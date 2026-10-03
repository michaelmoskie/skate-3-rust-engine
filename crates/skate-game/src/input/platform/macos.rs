//! macOS Game Controller adapter.
//!
//! Recent wired Xbox controllers expose vendor-defined HID reports on macOS.
//! Poll Apple's GameController framework instead of the raw HID backend so the
//! OS supplies its normalized extended-gamepad profile.
use super::{DeviceError, DevicePacket};
use objc2_game_controller::{GCController, GCControllerDirectionPad};
use skate_core::input::xbox::XboxState;

pub(crate) fn poll(index: usize, number: u32) -> Result<DevicePacket, DeviceError> {
    let controllers = unsafe { GCController::controllers() };
    let gamepad = controllers
        .iter()
        .filter_map(|controller| unsafe { controller.extendedGamepad() })
        .nth(index)
        .ok_or(DeviceError::Disconnected)?;

    let axis = |value: f32| {
        let value = value.clamp(-1.0, 1.0);
        (value * if value < 0.0 { 32768.0 } else { 32767.0 }).round() as i16
    };
    let direction = |pad: &GCControllerDirectionPad| unsafe { (pad.xAxis().value(), pad.yAxis().value()) };

    let dpad = unsafe { gamepad.dpad() };
    let (dpad_x, dpad_y) = direction(&dpad);
    let menu = unsafe { gamepad.buttonMenu() };
    let left_trigger = unsafe { gamepad.leftTrigger() };
    let right_trigger = unsafe { gamepad.rightTrigger() };
    let button_a = unsafe { gamepad.buttonA() };
    let button_b = unsafe { gamepad.buttonB() };
    let button_x = unsafe { gamepad.buttonX() };
    let button_y = unsafe { gamepad.buttonY() };
    let mut buttons = 0;
    for (active, mask) in [
        (dpad_y > 0.5, 0x0001),
        (dpad_y < -0.5, 0x0002),
        (dpad_x < -0.5, 0x0004),
        (dpad_x > 0.5, 0x0008),
        (unsafe { menu.isPressed() }, 0x0010),
        (unsafe { gamepad.buttonOptions() }.is_some_and(|button| unsafe { button.isPressed() }), 0x0020),
        (unsafe { gamepad.leftThumbstickButton() }.is_some_and(|button| unsafe { button.isPressed() }), 0x0040),
        (unsafe { gamepad.rightThumbstickButton() }.is_some_and(|button| unsafe { button.isPressed() }), 0x0080),
        (unsafe { left_trigger.value() } > 0.5, 0x0100),
        (unsafe { right_trigger.value() } > 0.5, 0x0200),
        (unsafe { button_a.isPressed() }, 0x1000),
        (unsafe { button_b.isPressed() }, 0x2000),
        (unsafe { button_x.isPressed() }, 0x4000),
        (unsafe { button_y.isPressed() }, 0x8000),
    ] {
        if active {
            buttons |= mask;
        }
    }
    let left = unsafe { gamepad.leftThumbstick() };
    let right = unsafe { gamepad.rightThumbstick() };
    let (left_x, left_y) = direction(&left);
    let (right_x, right_y) = direction(&right);
    Ok(DevicePacket {
        number,
        state: XboxState {
            buttons,
            triggers: [
                (unsafe { left_trigger.value().clamp(0.0, 1.0) } * 255.0).round() as u8,
                (unsafe { right_trigger.value().clamp(0.0, 1.0) } * 255.0).round() as u8,
            ],
            left: [axis(left_x), axis(left_y)],
            right: [axis(right_x), axis(right_y)],
        },
        subtype: 1,
    })
}
