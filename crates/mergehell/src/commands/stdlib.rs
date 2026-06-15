#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StandardModule {
    pub name: &'static str,
    pub description: &'static str,
}

const MODULES: &[StandardModule] = &[
    StandardModule {
        name: "rerere",
        description: "reuse recorded conflict resolutions",
    },
    StandardModule {
        name: "stash",
        description: "defer evaluation",
    },
    StandardModule {
        name: "blame",
        description: "choose using authorship metadata",
    },
    StandardModule {
        name: "bisect",
        description: "search over failure history",
    },
    StandardModule {
        name: "reset",
        description: "reset runtime state",
    },
    StandardModule {
        name: "reflog",
        description: "inspect previous runtime states",
    },
    StandardModule {
        name: "submodule",
        description: "track dependency regret",
    },
];

pub fn all_modules() -> &'static [StandardModule] {
    MODULES
}

pub fn find_module(name: &str) -> Option<StandardModule> {
    MODULES.iter().copied().find(|module| module.name == name)
}

pub fn is_standard_module(name: &str) -> bool {
    find_module(name).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_contains_required_modules() {
        let names = all_modules()
            .iter()
            .map(|module| module.name)
            .collect::<Vec<_>>();

        assert_eq!(
            names,
            vec![
                "rerere",
                "stash",
                "blame",
                "bisect",
                "reset",
                "reflog",
                "submodule"
            ]
        );
    }

    #[test]
    fn finds_standard_module_by_name() {
        assert_eq!(find_module("rerere").unwrap().name, "rerere");
        assert!(is_standard_module("blame"));
        assert!(!is_standard_module("vendor/math.mh"));
    }
}
