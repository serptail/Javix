use super::constants as cp;
use super::ClassFile;

pub fn resolve_interfaces(class: &ClassFile) -> Vec<String> {
    class
        .interfaces
        .iter()
        .filter_map(|index| cp::class_name(&class.constant_pool, *index).ok())
        .collect()
}
