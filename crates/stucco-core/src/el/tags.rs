//! One constructor per standard HTML element.

use super::{Element, VoidElement};

macro_rules! elements {
    ($($tag:ident)*) => {$(
        #[doc = concat!("A `<", stringify!($tag), ">` element.")]
        pub fn $tag<'a>() -> Element<'a> {
            Element::new(stringify!($tag))
        }
    )*};
}

macro_rules! void_elements {
    ($($tag:ident)*) => {$(
        #[doc = concat!("A void `<", stringify!($tag), ">` element.")]
        pub fn $tag() -> VoidElement {
            VoidElement::new(stringify!($tag))
        }
    )*};
}

elements! {
    a abbr address article aside audio b bdi bdo blockquote body button canvas
    caption cite code colgroup data datalist dd del details dfn dialog div dl dt
    em fieldset figcaption figure footer form h1 h2 h3 h4 h5 h6 head header
    hgroup html i iframe ins kbd label legend li main map mark menu meter nav
    noscript object ol optgroup option output p picture pre progress q rp rt
    ruby s samp script search section select slot small span strong style sub
    summary sup table tbody td template textarea tfoot th thead time title tr u
    ul var video
}

void_elements! {
    area base br col embed hr img input link meta source track wbr
}
