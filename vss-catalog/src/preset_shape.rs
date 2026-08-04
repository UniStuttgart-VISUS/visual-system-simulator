pub(crate) fn validate_preset_shape(
    has_both: bool,
    has_left: bool,
    has_right: bool,
) -> Result<(), &'static str> {
    let has_eye = has_left || has_right;
    match (has_both, has_eye) {
        (false, false) => Err("preset has no settings"),
        (true, true) => Err("preset mixes 'both' with eye-specific 'left' or 'right' settings"),
        _ => Ok(()),
    }
}
