use super::*;

#[test]
fn text_input_insert_and_backspace() {
    let mut input = TextInput::default();
    input.insert('a');
    input.insert('b');
    input.insert('c');
    assert_eq!(input.value, "abc");
    assert_eq!(input.cursor, 3);
    input.backspace();
    assert_eq!(input.value, "ab");
    assert_eq!(input.cursor, 2);
}

#[test]
fn text_input_insert_respects_cursor_position() {
    let mut input = TextInput {
        value: "ac".to_string(),
        cursor: 1,
    };
    input.insert('b');
    assert_eq!(input.value, "abc");
    assert_eq!(input.cursor, 2);
}

#[test]
fn text_input_backspace_at_start_is_noop() {
    let mut input = TextInput {
        value: "abc".to_string(),
        cursor: 0,
    };
    input.backspace();
    assert_eq!(input.value, "abc");
    assert_eq!(input.cursor, 0);
}

#[test]
fn text_input_left_right_clamp_at_bounds() {
    let mut input = TextInput {
        value: "ab".to_string(),
        cursor: 0,
    };
    input.left();
    assert_eq!(input.cursor, 0);
    input.right();
    input.right();
    input.right();
    assert_eq!(input.cursor, 2);
}

/// The whole reason `TextInput` tracks a char index instead of a byte index.
#[test]
fn text_input_handles_multibyte_chars() {
    let mut input = TextInput::default();
    input.insert('é');
    input.insert('a');
    assert_eq!(input.value, "éa");
    assert_eq!(input.cursor, 2);
    input.left();
    input.backspace();
    assert_eq!(input.value, "a");
}

#[test]
fn text_input_paste_strips_control_chars() {
    let mut input = TextInput::default();
    input.paste("hello\nworld\t!");
    assert_eq!(input.value, "helloworld!");
    assert_eq!(input.cursor, input.value.chars().count());
}

#[test]
fn text_input_clear_resets_value_and_cursor() {
    let mut input = TextInput {
        value: "abc".to_string(),
        cursor: 2,
    };
    input.clear();
    assert_eq!(input.value, "");
    assert_eq!(input.cursor, 0);
}

#[test]
fn duration_minutes_uses_preset_by_default() {
    let form = NewReservationFormState::default();
    assert_eq!(form.duration_minutes(), Some(60)); // duration_idx 2 -> "1 hr"
}

#[test]
fn duration_minutes_parses_valid_custom_value() {
    let mut form = NewReservationFormState {
        duration_idx: DURATION_PRESETS.len(),
        ..Default::default()
    };
    form.custom_minutes.value = "90".to_string();
    assert_eq!(form.duration_minutes(), Some(90));
}

#[test]
fn duration_minutes_rejects_zero_negative_and_non_numeric_custom_value() {
    let mut form = NewReservationFormState {
        duration_idx: DURATION_PRESETS.len(),
        ..Default::default()
    };
    for bad in ["0", "-5", "abc", ""] {
        form.custom_minutes.value = bad.to_string();
        assert_eq!(form.duration_minutes(), None, "expected None for {bad:?}");
    }
}

#[test]
fn visible_rows_hides_time_fields_when_starting_now() {
    let form = NewReservationFormState::default();
    assert_eq!(
        form.visible_rows(),
        vec![FormRow::Start, FormRow::Duration, FormRow::Create]
    );
}

#[test]
fn visible_rows_shows_time_fields_and_custom_minutes_when_needed() {
    let form = NewReservationFormState {
        start: StartChoice::Later,
        duration_idx: DURATION_PRESETS.len(),
        ..Default::default()
    };
    assert_eq!(
        form.visible_rows(),
        vec![
            FormRow::Start,
            FormRow::Day,
            FormRow::Hour,
            FormRow::Minute,
            FormRow::AmPm,
            FormRow::Duration,
            FormRow::CustomMinutes,
            FormRow::Create,
        ]
    );
}

#[test]
fn start_value_now_is_literal_now() {
    let form = NewReservationFormState::default();
    assert_eq!(form.start_value(), Some("now".to_string()));
}

#[test]
fn start_value_later_tomorrow_is_in_the_future() {
    let form = NewReservationFormState {
        start: StartChoice::Later,
        day_offset: 1,
        ..Default::default()
    };
    let value = form
        .start_value()
        .expect("noon tomorrow should never be a DST gap");
    let ts: i64 = value.parse().unwrap();
    assert!(ts > Local::now().timestamp());
}

fn image_fixture(id: i64, name: &str) -> Image {
    Image {
        id,
        name: name.to_string(),
        ostype: String::new(),
        usage: String::new(),
        description: String::new(),
    }
}

#[test]
fn visible_indices_filters_case_insensitively() {
    let mut images = ImagesUiState {
        images: vec![
            image_fixture(1, "Ubuntu 22.04"),
            image_fixture(2, "Windows 11"),
        ],
        ..Default::default()
    };
    images.search.value = "ubuntu".to_string();
    assert_eq!(images.visible_indices(), vec![0]);
}

#[test]
fn visible_indices_returns_everything_when_search_is_empty() {
    let images = ImagesUiState {
        images: vec![
            image_fixture(1, "Ubuntu 22.04"),
            image_fixture(2, "Windows 11"),
        ],
        ..Default::default()
    };
    assert_eq!(images.visible_indices(), vec![0, 1]);
}
