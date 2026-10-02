//! Forms (feature `forms`): form structure, controls and `Field`, which wires
//! labels, hints and errors and binds to a submitted [`stucco_core::FormState`].

use stucco_core::{Asset, register_asset};

mod structure;

pub use structure::{CsrfToken, FieldError, FieldHint, Fieldset, Form, HiddenInput, Legend};

#[cfg(test)]
mod tests;

/// The forms family's stylesheet.
pub static FORMS: Asset = Asset {
    name: "st-forms",
    css: Some(include_str!("../../css/forms.css")),
    behavior: None,
    deps: &[],
};
register_asset!(FORMS);
