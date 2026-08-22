//! Folding the spellings the kernel uses for one thing into one.

///
/// Kernels differ in case, in whether they separate words with a space,
/// a dash or an underscore, and in the numeric suffix they
/// append to the second instance of a chip, so every spelling
/// folds onto one entry instead of the tables listing them all.
#[must_use]
pub fn normalise(value: &str) -> String {
    let mut folded = String::with_capacity(value.len());
    let mut pending_space = false;

    for character in value.chars() {
        if character.is_whitespace() || character == '_' || character == '-' {
            pending_space = !folded.is_empty();
            continue;
        }

        if pending_space {
            folded.push(' ');
            pending_space = false;
        }

        folded.extend(character.to_lowercase());
    }

    folded
}

/// Folds a chip name and drops the instance number the kernel appends.
///
/// A second chip of the same family arrives as `acpitz_1`, and a
/// machine with two processor packages arrives as `coretemp`
/// twice, so the number belongs to the instance rather than to
/// the family.
#[must_use]
pub fn normalise_chip(chip: &str) -> String {
    let folded = normalise(chip);

    match folded.rsplit_once(' ') {
        Some((base, suffix))
            if !base.is_empty()
                && !suffix.is_empty()
                && suffix.bytes().all(|b| b.is_ascii_digit()) =>
        {
            base.to_owned()
        }
        _ => folded
    }
}

/// Reports whether an input label carries no information about what it
/// measures.
///
/// A driver that labels nothing leaves `tempN_label` absent, and the
/// kernel interface names the file itself `tempN`, so both
/// forms mean the same.
#[must_use]
pub fn is_unnamed_input(input: &str) -> bool {
    let folded = normalise(input);

    folded.is_empty()
        || folded
            .strip_prefix("temp")
            .is_some_and(|rest| rest.bytes().all(|b| b.is_ascii_digit()))
}
