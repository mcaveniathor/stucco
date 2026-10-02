//! Assets components declare while rendering, and their resolution (spec §4.6).

use std::fmt;

/// A stylesheet chunk and/or client behaviour a component needs.
///
/// Declare one as a `static`, register it with [`register_asset!`] so
/// `Bundle` serves it, and call `cx.require(&ASSET)` when rendering.
pub struct Asset {
    /// Unique name; used in file names.
    pub name: &'static str,
    /// CSS, inside a `stucco.*` cascade layer.
    pub css: Option<&'static str>,
    /// Client-side enhancement.
    pub behavior: Option<Behavior>,
    /// Assets this one needs first.
    pub deps: &'static [&'static Asset],
}

impl fmt::Debug for Asset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let deps: Vec<_> = self.deps.iter().map(|d| d.name).collect();
        f.debug_struct("Asset")
            .field("name", &self.name)
            .field("css", &self.css.is_some())
            .field("behavior", &self.behavior.is_some())
            .field("deps", &deps)
            .finish()
    }
}

/// Client-side code attached to an [`Asset`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Behavior {
    /// A dependency-free ES module.
    Js(&'static str),
}

/// The resolved, dependency-ordered assets a render required.
#[derive(Debug, Clone, Default)]
pub struct AssetRequirements {
    list: Vec<&'static Asset>,
}

impl AssetRequirements {
    /// Resolves `requested` depth-first, dependencies first, each asset once.
    pub(crate) fn resolve(requested: &[&'static Asset]) -> AssetRequirements {
        fn visit(
            asset: &'static Asset,
            visiting: &mut Vec<&'static Asset>,
            out: &mut Vec<&'static Asset>,
        ) {
            let seen = |list: &[&'static Asset]| list.iter().any(|a| std::ptr::eq(*a, asset));
            if seen(out) || seen(visiting) {
                return;
            }
            visiting.push(asset);
            for dep in asset.deps {
                visit(dep, visiting, out);
            }
            visiting.pop();
            out.push(asset);
        }
        let mut out = Vec::new();
        let mut visiting = Vec::new();
        for asset in requested {
            visit(asset, &mut visiting, &mut out);
        }
        AssetRequirements { list: out }
    }

    /// The assets in dependency order.
    pub fn iter(&self) -> impl Iterator<Item = &'static Asset> + '_ {
        self.list.iter().copied()
    }

    /// The assets that carry a behaviour.
    pub fn behaviors(&self) -> impl Iterator<Item = &'static Asset> + '_ {
        self.iter().filter(|a| a.behavior.is_some())
    }

    /// Whether nothing was required.
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    #[allow(dead_code)] // used by fragments (Task 12)
    pub(crate) fn retain(&mut self, keep: impl FnMut(&&'static Asset) -> bool) {
        self.list.retain(keep);
    }
}

/// A registry entry; created by [`register_asset!`].
#[derive(Debug)]
pub struct AssetRef(pub &'static Asset);

inventory::collect!(AssetRef);

/// Registers a `static` [`Asset`] so `Bundle` serves its CSS and behaviour.
#[macro_export]
macro_rules! register_asset {
    ($asset:path) => {
        $crate::inventory::submit! { $crate::AssetRef(&$asset) }
    };
}

/// Every registered asset, sorted by name.
pub fn registered_assets() -> Vec<&'static Asset> {
    let mut all: Vec<_> = inventory::iter::<AssetRef>
        .into_iter()
        .map(|r| r.0)
        .collect();
    all.sort_by_key(|a| a.name);
    all
}

/// Whether `asset` was registered with [`register_asset!`].
pub fn is_registered(asset: &'static Asset) -> bool {
    inventory::iter::<AssetRef>
        .into_iter()
        .any(|r| std::ptr::eq(r.0, asset))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Cx;

    static D: Asset = Asset {
        name: "d",
        css: Some(".d{}"),
        behavior: None,
        deps: &[],
    };
    static B: Asset = Asset {
        name: "b",
        css: None,
        behavior: None,
        deps: &[&D],
    };
    static C: Asset = Asset {
        name: "c",
        css: None,
        behavior: None,
        deps: &[&D],
    };
    static A: Asset = Asset {
        name: "a",
        css: None,
        behavior: None,
        deps: &[&B, &C],
    };
    static X: Asset = Asset {
        name: "x",
        css: None,
        behavior: None,
        deps: &[&Y],
    };
    static Y: Asset = Asset {
        name: "y",
        css: None,
        behavior: None,
        deps: &[&X],
    };
    crate::register_asset!(D);

    fn names(r: &AssetRequirements) -> Vec<&'static str> {
        r.iter().map(|a| a.name).collect()
    }

    #[test]
    fn diamonds_resolve_once_dependencies_first() {
        let mut cx = Cx::new();
        cx.require(&A);
        cx.require(&D);
        assert_eq!(names(&cx.finish().1), ["d", "b", "c", "a"]);
    }

    #[test]
    fn cycles_terminate() {
        let mut cx = Cx::new();
        cx.require(&X);
        assert_eq!(names(&cx.finish().1), ["y", "x"]);
    }

    #[test]
    fn registry_finds_registered_assets_only() {
        assert!(is_registered(&D) && !is_registered(&A));
        assert!(registered_assets().iter().any(|a| a.name == "d"));
    }
}
