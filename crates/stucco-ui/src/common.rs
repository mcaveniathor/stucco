//! Shared vocabulary: spacing, sizes, measures, variants and tones. Each value
//! renders as a closed set of data-attribute values (never inline styles).

macro_rules! data_enum {
    ($(#[$doc:meta])* $name:ident { $($(#[$vdoc:meta])* $variant:ident => $value:literal),+ $(,)? }) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum $name {
            $($(#[$vdoc])* $variant),+
        }

        impl $name {
            /// The data-attribute value.
            pub fn as_str(self) -> &'static str {
                match self {
                    $($name::$variant => $value),+
                }
            }
        }
    };
}

data_enum! {
    /// A step of the theme's spacing scale (`--st-space-N`).
    Space {
        /// No space.
        S0 => "0",
        /// `--st-space-1`.
        S1 => "1",
        /// `--st-space-2`.
        S2 => "2",
        /// `--st-space-3`.
        S3 => "3",
        /// `--st-space-4`.
        S4 => "4",
        /// `--st-space-5`.
        S5 => "5",
        /// `--st-space-6`.
        S6 => "6",
        /// `--st-space-8`.
        S8 => "8",
        /// `--st-space-10`.
        S10 => "10",
        /// `--st-space-12`.
        S12 => "12",
    }
}

data_enum! {
    /// A visual size step.
    Size {
        /// Extra small.
        Xs => "xs",
        /// Small.
        Sm => "sm",
        /// Medium (body).
        Md => "md",
        /// Large.
        Lg => "lg",
        /// Extra large.
        Xl => "xl",
        /// 2× extra large.
        Xl2 => "2xl",
        /// 3× extra large.
        Xl3 => "3xl",
    }
}

data_enum! {
    /// A width: 20, 30, 40, 60 or 75rem, or a comfortable reading measure (65ch).
    Measure {
        /// 20rem.
        Xs => "xs",
        /// 30rem.
        Sm => "sm",
        /// 40rem.
        Md => "md",
        /// 60rem.
        Lg => "lg",
        /// 75rem.
        Xl => "xl",
        /// 65ch, for running text.
        Prose => "prose",
    }
}

data_enum! {
    /// Emphasis of an action.
    Variant {
        /// The main action.
        Primary => "primary",
        /// A normal action.
        Secondary => "secondary",
        /// A low-emphasis action.
        Ghost => "ghost",
        /// A destructive action.
        Danger => "danger",
    }
}

data_enum! {
    /// Text and status colour.
    Tone {
        /// Body text colour.
        Default => "default",
        /// Secondary text.
        Muted => "muted",
        /// Accent colour.
        Accent => "accent",
        /// Success.
        Success => "success",
        /// Warning.
        Warning => "warning",
        /// Danger.
        Danger => "danger",
        /// Information.
        Info => "info",
    }
}

data_enum! {
    /// A forced colour scheme.
    ColorScheme {
        /// Light.
        Light => "light",
        /// Dark.
        Dark => "dark",
    }
}
