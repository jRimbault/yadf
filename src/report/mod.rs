mod display;

use crate::bag::TreeBag;
use crate::fs::wrapper::DirEntry;
use byte_unit::Byte;
use num_format::Locale;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

/// Extract informations from a [`TreeBag`](struct.TreeBag.html) of [`DirEntry`](struct.DirEntry.html),
/// for the purpose of displaying it to a reader.
#[derive(Debug)]
pub struct Report {
    locale: Locale,
    number_uniques: usize,
    duplicate_groups: usize,
    number_duplicates: usize,
    uniques_bytes: byte_unit::Byte,
    duplicate_bytes: byte_unit::Byte,
}

impl<H: Ord> From<&TreeBag<H, DirEntry>> for Report {
    fn from(bag: &TreeBag<H, DirEntry>) -> Self {
        Self::with_locale(bag, Locale::en)
    }
}

impl Report {
    fn total_files(&self) -> usize {
        self.number_uniques + self.number_duplicates
    }

    fn total_size(&self) -> byte_unit::Byte {
        Byte::from_bytes(self.uniques_bytes.get_bytes() + self.duplicate_bytes.get_bytes())
    }

    /// Extract informations from the bag, with the given `Locale` it will display
    /// the information with the correct formatting.
    pub fn with_locale<H: Ord>(bag: &TreeBag<H, DirEntry>, locale: Locale) -> Self {
        let (duplicates, uniques): (Vec<_>, Vec<_>) =
            bag.values().partition(|bucket| bucket.len() > 1);
        let uniques: Vec<_> = uniques.into_iter().flatten().collect();
        let number_uniques = uniques.len();
        let uniques_bytes: u64 = uniques
            .into_par_iter()
            .filter_map(|e| Some(e.metadata().ok()?.len()))
            .sum();
        let duplicate_groups = duplicates.len();
        let number_duplicates = duplicates.iter().map(|d| d.len()).sum();
        let duplicate_bytes: u64 = duplicates
            .into_par_iter()
            .flatten()
            .filter_map(|e| Some(e.metadata().ok()?.len()))
            .sum();
        Self {
            locale,
            number_uniques,
            number_duplicates,
            uniques_bytes: Byte::from_bytes(uniques_bytes as _),
            duplicate_bytes: Byte::from_bytes(duplicate_bytes as _),
            duplicate_groups,
        }
    }
}

impl Default for Report {
    fn default() -> Self {
        Self {
            locale: Locale::en,
            number_uniques: 0,
            number_duplicates: 0,
            uniques_bytes: Byte::from_bytes(0),
            duplicate_bytes: Byte::from_bytes(0),
            duplicate_groups: 0,
        }
    }
}
