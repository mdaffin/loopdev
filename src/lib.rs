use std::fs;

pub struct LoopDevices {
    it: fs::ReadDir,
}

impl Iterator for LoopDevices {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        None
        // self.it.next().map(|d| d)
    }
}

pub fn list() {
    let paths = fs::read_dir("/dev").unwrap();
    for path in paths.filter_map(|entry| {
        entry.ok().and_then(|e| {
            match e.path().to_str() {
                Some(n) if n.starts_with("/dev/loop") => Some(String::from(n)),
                _ => None,
            }

        })
    }) {
        println!("Name: {}", path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_all_test() {
        list();
    }
}
