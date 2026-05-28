#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModulePath {
    segments: Vec<String>,
}

impl ModulePath {
    pub fn root(crate_name: impl Into<String>) -> Self {
        Self {
            segments: vec![crate_name.into()],
        }
    }

    pub fn child(&self, module: impl Into<String>) -> Self {
        let mut segments = self.segments.clone();
        segments.push(module.into());
        Self { segments }
    }

    pub(crate) fn qualified(&self, ident: &str) -> String {
        let mut path = self.segments.join("::");
        path.push_str("::");
        path.push_str(ident);
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_qualifies_items_under_the_crate_segment() {
        assert_eq!(ModulePath::root("demo").qualified("add"), "demo::add");
    }

    #[test]
    fn child_paths_preserve_all_ancestors_in_order() {
        let path = ModulePath::root("demo").child("geometry").child("point");

        assert_eq!(path.qualified("Point"), "demo::geometry::point::Point");
    }

    #[test]
    fn child_does_not_mutate_the_parent_path() {
        let parent = ModulePath::root("demo");
        let child = parent.child("geometry");

        assert_eq!(parent.qualified("Point"), "demo::Point");
        assert_eq!(child.qualified("Point"), "demo::geometry::Point");
    }
}
