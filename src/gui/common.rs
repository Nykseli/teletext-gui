use egui::{self, InputState, Key::*};

const NUM_KEYS: [egui::Key; 10] = [Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9];

/// Return None if number is not pressed
pub fn input_to_num(input: &InputState) -> Option<i32> {
    for (idx, key) in NUM_KEYS.iter().enumerate() {
        if input.key_released(*key) {
            return Some(idx as i32);
        }
    }

    None
}
