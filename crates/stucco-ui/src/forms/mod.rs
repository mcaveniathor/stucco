//! Forms (feature `forms`): form structure, controls and `Field`, which wires
//! labels, hints and errors and binds to a submitted [`stucco_core::FormState`].

use stucco_core::{Asset, register_asset};

mod choice;
mod control;
mod field;
mod input;
mod select;
mod structure;
mod textarea;

pub use choice::{Checkbox, RadioGroup};
pub use control::{Control, Wiring};
pub use field::Field;
pub use input::Input;
pub use select::Select;
pub use structure::{CsrfToken, FieldError, FieldHint, Fieldset, Form, HiddenInput, Legend};
pub use textarea::Textarea;

#[cfg(test)]
mod control_tests;
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
