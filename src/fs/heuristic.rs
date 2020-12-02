use once_cell::sync::Lazy;
use std::path::{Path, PathBuf};
use sysinfo::{DiskExt, DiskType, SystemExt};

#[cfg(not(windows))]
use unix::disk_type;
#[cfg(windows)]
use win::disk_type;

pub fn num_cpus_get<P: AsRef<Path>>(paths: &[P]) -> usize {
    let (ssds, others): (Vec<DiskType>, Vec<DiskType>) = paths
        .iter()
        .map(disk_type)
        .partition(|&disk_type| disk_type == DiskType::SSD);
    // study a better heuristics here,
    // unfortunately I don't have any internal HDDs to test things with
    if ssds.len() > others.len() {
        num_cpus::get()
    } else {
        num_cpus::get() / 2
    }
}

#[derive(Debug)]
struct Disk {
    path: PathBuf,
    disk_type: DiskType,
}

impl From<(PathBuf, DiskType)> for Disk {
    fn from((path, disk_type): (PathBuf, DiskType)) -> Self {
        Self { path, disk_type }
    }
}

fn find_disk(path: &Path, disks: &[Disk]) -> Option<DiskType> {
    disks
        .iter()
        .find(|disk| path.starts_with(&disk.path))
        .map(|disk| disk.disk_type)
}

#[allow(dead_code)]
mod win {
    use super::*;

    #[derive(Debug)]
    struct System {
        mounted: Vec<Disk>,
    }

    static SYSTEM: Lazy<System> = Lazy::new(|| System {
        mounted: sysinfo::System::new_all()
            .get_disks()
            .iter()
            .map(|disk| (disk.get_mount_point().to_owned(), disk.get_type()))
            .map(Into::into)
            .collect(),
    });

    pub fn disk_type<P: AsRef<Path>>(path: &P) -> DiskType {
        let path = dunce::canonicalize(path.as_ref()).unwrap();
        find_disk(&path, &SYSTEM.mounted).unwrap_or(DiskType::Unknown(-1))
    }
}

#[allow(dead_code)]
mod unix {
    use super::*;

    #[derive(Debug)]
    struct System {
        root: Disk,
        mounted: Vec<Disk>,
    }

    static SYSTEM: Lazy<System> = Lazy::new(|| {
        use std::path::Component;
        let infos = sysinfo::System::new_all();
        let mut system = System {
            root: ("".into(), DiskType::Unknown(-1)).into(),
            mounted: Vec::new(),
        };
        for disk in infos.get_disks() {
            let mut components = disk.get_mount_point().components();
            if let (Some(Component::RootDir), None) = (components.next(), components.next()) {
                system.root = (disk.get_mount_point().to_owned(), disk.get_type()).into();
                continue;
            }
            system
                .mounted
                .push((disk.get_mount_point().to_owned(), disk.get_type()).into());
        }
        system
    });

    pub fn disk_type<P: AsRef<Path>>(path: &P) -> DiskType {
        let path = dunce::canonicalize(path.as_ref()).unwrap();
        find_disk(&path, &SYSTEM.mounted).unwrap_or_else(|| {
            if path.starts_with(&SYSTEM.root.path) {
                SYSTEM.root.disk_type
            } else {
                DiskType::Unknown(-1)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn exploring() {
        let paths = &[
            dirs::home_dir().unwrap(),
            #[cfg(windows)]
            r#"\\IRIDIUM\Plex-DoubleSloth"#.into(),
            r#"../.."#.into(),
        ];
        for path in paths {
            let dtype = disk_type(path);
            println!("{:?} : {:?}", path, dtype);
        }
        println!("{}", num_cpus_get(paths));
    }
}
