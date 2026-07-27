//! Fan-in: many workers producing key/value pairs, one collector draining
//! them into a [`TreeBag`].
//!
//! Every phase of the scan has this shape -- the walker grouping paths by
//! size, the partial pass grouping them by prefix hash, the content pass
//! grouping them by full hash -- so the channel, the collector task and the
//! "what do we do if the channel is closed" answer live here once.

use crate::TreeBag;

/// Bound on how many finished pairs may sit in flight. Wide enough that
/// workers rarely block on the collector, bounded so a fast producer can't
/// grow the queue without limit.
const CHANNEL_SIZE: usize = 8 * 1024;

/// Where a worker hands finished pairs to the collector.
#[derive(Debug)]
pub struct Sink<K, V>(crossbeam_channel::Sender<(K, V)>);

impl<K, V> Sink<K, V> {
    /// Hands one pair to the collector.
    ///
    /// A closed channel means the collector is gone, which can't happen
    /// while [`collect`] is running; logging beats panicking in a worker
    /// thread if it ever does.
    pub fn send(&self, key: K, value: V) {
        if let Err(error) = self.0.send((key, value)) {
            log::error!("{}, couldn't send value across channel", error);
        }
    }
}

/// Hand-written: the derived impl would demand `K: Clone, V: Clone`, which
/// the channel sender doesn't actually need.
impl<K, V> Clone for Sink<K, V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

/// Runs `produce` alongside a collector draining its output into a bag.
///
/// Returns once `produce` has returned and every clone of its [`Sink`] has
/// been dropped, which is what closes the channel and ends the collector.
pub fn collect<K, V>(produce: impl FnOnce(Sink<K, V>) + Send) -> TreeBag<K, V>
where
    K: Ord + Send,
    V: Send,
{
    let (sender, receiver) = crossbeam_channel::bounded(CHANNEL_SIZE);
    rayon::join(
        move || receiver.into_iter().collect(),
        move || produce(Sink(sender)),
    )
    .0
}
