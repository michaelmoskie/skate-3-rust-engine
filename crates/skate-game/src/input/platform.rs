//! Device transport. Windows preserves raw XInput samples; other platforms
//! quantize Bevy's standardized gamepad state into the same TU3 input packet.
use skate_core::input::xbox::XboxState;

#[cfg(target_os = "macos")]
pub(crate) mod macos;

pub(crate) struct DevicePacket {
    pub number: u32,
    pub state: XboxState,
    pub subtype: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DeviceError {
    Disconnected,
    #[cfg(windows)]
    State(u32),
    #[cfg(any(windows, test))]
    Capabilities(u32),
}

/// Device identity is metadata; raw input is still sampled every host frame.
/// Refresh periodically as well as after errors, so hot swaps cannot leave a
/// subtype cached indefinitely even if Windows never exposes a disconnect.
#[derive(Default)]
pub(crate) struct CapabilityCache {
    value: Option<(u8, std::time::Instant)>,
}
impl CapabilityCache {
    pub(crate) fn invalidate(&mut self) {
        self.value = None;
    }
    #[cfg(windows)]
    fn get(
        &mut self,
        now: std::time::Instant,
        read: impl FnOnce() -> Result<u8, DeviceError>,
    ) -> Result<u8, DeviceError> {
        if let Some((subtype, expires)) = self.value {
            if now < expires {
                return Ok(subtype);
            }
        }
        self.value = None;
        let subtype = read()?;
        self.value = Some((subtype, now + std::time::Duration::from_secs(1)));
        Ok(subtype)
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::mem::MaybeUninit;

    // ABI from the installed Windows SDK Xinput.h. No OS-owned pointers are
    // retained and only successful calls permit reading output storage.
    #[repr(C)]
    struct Gamepad {
        buttons: u16,
        left_trigger: u8,
        right_trigger: u8,
        left_x: i16,
        left_y: i16,
        right_x: i16,
        right_y: i16,
    }
    #[repr(C)]
    struct State {
        number: u32,
        gamepad: Gamepad,
    }
    #[repr(C)]
    struct Vibration {
        left: u16,
        right: u16,
    }
    #[repr(C)]
    struct Capabilities {
        device_type: u8,
        subtype: u8,
        flags: u16,
        gamepad: Gamepad,
        vibration: Vibration,
    }
    const _: () = assert!(size_of::<Gamepad>() == 12);
    const _: () = assert!(size_of::<State>() == 16);
    const _: () = assert!(size_of::<Capabilities>() == 20);

    #[link(name = "xinput")]
    unsafe extern "system" {
        fn XInputGetState(index: u32, state: *mut State) -> u32;
        fn XInputGetCapabilities(index: u32, flags: u32, capabilities: *mut Capabilities) -> u32;
    }

    pub(super) fn poll(
        index: u32,
        cache: &mut CapabilityCache,
    ) -> Result<DevicePacket, DeviceError> {
        let mut state = MaybeUninit::<State>::uninit();
        // SAFETY: properly aligned writable storage with the SDK's exact C ABI.
        let result = unsafe { XInputGetState(index, state.as_mut_ptr()) };
        if result != 0 {
            cache.invalidate();
        }
        if result == 1167 {
            return Err(DeviceError::Disconnected);
        }
        if result != 0 {
            return Err(DeviceError::State(result));
        }
        let subtype = cache.get(std::time::Instant::now(), || {
            let mut capabilities = MaybeUninit::<Capabilities>::uninit();
            // SAFETY: writable storage with the SDK ABI; read only on success.
            let result = unsafe { XInputGetCapabilities(index, 1, capabilities.as_mut_ptr()) };
            if result != 0 {
                return Err(DeviceError::Capabilities(result));
            }
            Ok(unsafe { capabilities.assume_init() }.subtype)
        })?;
        // SAFETY: successful XInputGetState initialized the complete structure.
        let state = unsafe { state.assume_init() };
        Ok(DevicePacket {
            number: state.number,
            state: XboxState {
                buttons: state.gamepad.buttons,
                triggers: [state.gamepad.left_trigger, state.gamepad.right_trigger],
                left: [state.gamepad.left_x, state.gamepad.left_y],
                right: [state.gamepad.right_x, state.gamepad.right_y],
            },
            subtype,
        })
    }
}

#[cfg(windows)]
pub(crate) fn poll_cached(
    index: usize,
    cache: &mut CapabilityCache,
) -> Result<DevicePacket, DeviceError> {
    assert!(index < 4);
    windows::poll(index as u32, cache)
}

#[cfg(not(windows))]
pub(crate) fn from_gamepad(gamepad: &bevy::input::gamepad::Gamepad, number: u32) -> DevicePacket {
    use bevy::input::gamepad::{GamepadAxis as Axis, GamepadButton as Button};

    let pressed = |button| gamepad.pressed(button);
    let mut buttons = 0;
    for (button, mask) in [
        (Button::DPadUp, 0x0001),
        (Button::DPadDown, 0x0002),
        (Button::DPadLeft, 0x0004),
        (Button::DPadRight, 0x0008),
        (Button::Start, 0x0010),
        (Button::Select, 0x0020),
        (Button::LeftThumb, 0x0040),
        (Button::RightThumb, 0x0080),
        (Button::LeftTrigger, 0x0100),
        (Button::RightTrigger, 0x0200),
        (Button::South, 0x1000),
        (Button::East, 0x2000),
        (Button::West, 0x4000),
        (Button::North, 0x8000),
    ] {
        if pressed(button) {
            buttons |= mask;
        }
    }
    let trigger = |button| {
        (gamepad.get(button).unwrap_or(0.0).clamp(0.0, 1.0) * 255.0).round() as u8
    };
    let axis = |axis| {
        let value = gamepad.get(axis).unwrap_or(0.0).clamp(-1.0, 1.0);
        (value * if value < 0.0 { 32768.0 } else { 32767.0 }).round() as i16
    };
    DevicePacket {
        number,
        state: XboxState {
            buttons,
            triggers: [trigger(Button::LeftTrigger2), trigger(Button::RightTrigger2)],
            left: [axis(Axis::LeftStickX), axis(Axis::LeftStickY)],
            right: [axis(Axis::RightStickX), axis(Axis::RightStickY)],
        },
        subtype: 1,
    }
}

#[cfg(all(test, windows))]
mod cache_tests {
    use super::*;
    #[test]
    fn capability_cache_refreshes_and_never_caches_errors() {
        let start = std::time::Instant::now();
        let mut cache = CapabilityCache::default();
        assert_eq!(cache.get(start, || Ok(1)), Ok(1));
        assert_eq!(
            cache.get(start + std::time::Duration::from_millis(999), || panic!(
                "redundant capability query"
            )),
            Ok(1)
        );
        assert_eq!(
            cache.get(start + std::time::Duration::from_secs(1), || Ok(2)),
            Ok(2)
        );
        cache.invalidate();
        assert_eq!(
            cache.get(start, || Err(DeviceError::Capabilities(5))),
            Err(DeviceError::Capabilities(5))
        );
        assert_eq!(cache.get(start, || Ok(3)), Ok(3));
        cache.invalidate();
        assert_eq!(cache.get(start, || Ok(4)), Ok(4));
    }
}

#[cfg(all(test, not(windows)))]
mod portable_tests {
    use super::*;
    use bevy::input::gamepad::{Gamepad, GamepadAxis as Axis, GamepadButton as Button};

    #[test]
    fn standardized_gamepad_is_quantized_as_xinput() {
        let mut gamepad = Gamepad::default();
        gamepad.digital_mut().press(Button::South);
        gamepad.digital_mut().press(Button::DPadLeft);
        gamepad.analog_mut().set(Button::LeftTrigger2, 0.5);
        gamepad.analog_mut().set(Axis::LeftStickX, -1.0);
        gamepad.analog_mut().set(Axis::RightStickY, 1.0);
        let packet = from_gamepad(&gamepad, 7);
        assert_eq!(packet.number, 7);
        assert_eq!(packet.state.buttons, 0x1004);
        assert_eq!(packet.state.triggers, [128, 0]);
        assert_eq!(packet.state.left, [i16::MIN, 0]);
        assert_eq!(packet.state.right, [0, i16::MAX]);
    }
}

// Preserve the uncached API for menu-only polling.
#[cfg(windows)]
pub(crate) fn poll(index: usize) -> Result<DevicePacket, DeviceError> {
    poll_cached(index, &mut CapabilityCache::default())
}
