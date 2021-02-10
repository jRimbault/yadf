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
        .map(|path| (path, disk_type(path)))
        .inspect(|(path, disk_type)| log::debug!("{:?} may be on a {:?}", path.as_ref(), disk_type))
        .map(|t| t.1)
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

/// logging wrapper for dunce
fn canonicalize<P: AsRef<Path>>(path: &P) -> std::io::Result<PathBuf> {
    match dunce::canonicalize(path.as_ref()) {
        Err(error) => {
            log::warn!(
                "{}, couldn't resolve path {:?} to a canonical path",
                error,
                path.as_ref()
            );
            Err(error)
        }
        Ok(v) => Ok(v),
    }
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
        let abs_path = match canonicalize(path) {
            Err(_) => return DiskType::Unknown(-1),
            Ok(v) => v,
        };
        log::trace!("path {:?} canonicalized to : {:?}", path.as_ref(), abs_path);
        find_disk(&abs_path, &SYSTEM.mounted).unwrap_or(DiskType::Unknown(-1))
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
        let abs_path = match canonicalize(path) {
            Err(_) => return DiskType::Unknown(-1),
            Ok(v) => v,
        };
        log::trace!("path {:?} canonicalized to : {:?}", path.as_ref(), abs_path);
        find_disk(&abs_path, &SYSTEM.mounted).unwrap_or_else(|| {
            if abs_path.starts_with(&SYSTEM.root.path) {
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
        env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Trace)
            .init();
        let paths = &[
            dirs::home_dir().unwrap(),
            #[cfg(windows)]
            r#"\\IRIDIUM\Plex-DoubleSloth"#.into(),
            r#"../.."#.into(),
        ];
        println!("{}", num_cpus_get(paths));
    }

    #[test]
    #[ignore]
    fn what_the_disk() {
        env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Trace)
            .init();
        let path = "../..";
        let disk_type = disk_type(&path);
        println!("{:?} is on a {:?}", path, disk_type);
    }
}
