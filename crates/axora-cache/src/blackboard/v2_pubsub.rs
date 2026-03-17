//! Publish/subscribe support for Blackboard v2.

use super::v2_versioning::Update;
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

/// Stable identifier for a subscription.
pub type SubscriptionId = u64;

/// Trait implemented by Blackboard v2 subscribers.
pub trait Subscriber: Send + Sync {
    /// Handles an emitted update.
    fn notify(&self, update: Update);
}

/// Stored subscription descriptor.
#[derive(Clone)]
pub struct Subscription {
    /// Stable subscription id.
    pub id: SubscriptionId,
    subscriber: Arc<dyn Subscriber>,
}

/// Concurrent publish/subscribe registry.
#[derive(Default)]
pub struct PubSubHub {
    subscribers: DashMap<String, Vec<Subscription>>,
    next_subscription_id: AtomicU64,
}

impl PubSubHub {
    /// Creates an empty hub.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a trait-based subscriber for a key.
    pub fn subscribe<S>(&self, key: &str, subscriber: S) -> SubscriptionId
    where
        S: Subscriber + 'static,
    {
        self.subscribe_arc(key, Arc::new(subscriber))
    }

    /// Registers an already boxed subscriber for a key.
    pub fn subscribe_arc(&self, key: &str, subscriber: Arc<dyn Subscriber>) -> SubscriptionId {
        let id = self.next_subscription_id.fetch_add(1, Ordering::SeqCst) + 1;
        let subscription = Subscription { id, subscriber };

        self.subscribers
            .entry(key.to_string())
            .or_default()
            .push(subscription);

        id
    }

    /// Registers a channel-based subscriber and returns its receiver.
    pub fn subscribe_channel(&self, key: &str) -> (SubscriptionId, UnboundedReceiver<Update>) {
        let (sender, receiver) = unbounded_channel();
        let id = self.subscribe(key, ChannelSubscriber { sender });
        (id, receiver)
    }

    /// Publishes an update to every subscriber for a key.
    pub fn publish(&self, key: &str, update: Update) -> usize {
        let subscribers = self
            .subscribers
            .get(key)
            .map(|entries| entries.clone())
            .unwrap_or_default();

        for subscription in &subscribers {
            subscription.subscriber.notify(update.clone());
        }

        subscribers.len()
    }

    /// Unregisters a subscription by id.
    pub fn unsubscribe(&self, id: SubscriptionId) -> bool {
        let keys: Vec<String> = self
            .subscribers
            .iter()
            .map(|entry| entry.key().clone())
            .collect();

        for key in keys {
            let mut removed = false;
            let mut should_drop_key = false;

            if let Some(mut subscriptions) = self.subscribers.get_mut(&key) {
                let before = subscriptions.len();
                subscriptions.retain(|subscription| subscription.id != id);
                removed = before != subscriptions.len();
                should_drop_key = subscriptions.is_empty();
            }

            if should_drop_key {
                self.subscribers.remove(&key);
            }

            if removed {
                return true;
            }
        }

        false
    }

    /// Returns the number of subscribers for a key.
    pub fn subscriber_count(&self, key: &str) -> usize {
        self.subscribers
            .get(key)
            .map(|entry| entry.len())
            .unwrap_or(0)
    }
}

struct ChannelSubscriber {
    sender: UnboundedSender<Update>,
}

impl Subscriber for ChannelSubscriber {
    fn notify(&self, update: Update) {
        let _ = self.sender.send(update);
    }
}

#[cfg(test)]
mod tests {
    use super::{PubSubHub, Subscriber};
    use crate::blackboard::v2::v2_versioning::Update;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct RecordingSubscriber {
        updates: Mutex<Vec<Update>>,
    }

    impl RecordingSubscriber {
        fn updates(&self) -> Vec<Update> {
            self.updates.lock().unwrap().clone()
        }
    }

    impl Subscriber for RecordingSubscriber {
        fn notify(&self, update: Update) {
            self.updates.lock().unwrap().push(update);
        }
    }

    fn sample_update(version: u64) -> Update {
        Update {
            key: "task".to_string(),
            old_value: Some(json!({"status": "draft"})),
            new_value: json!({"status": "done"}),
            diff: json!({"status": {"$replace": "done"}}),
            version,
            timestamp: 123,
            full_size_bytes: 18,
            diff_size_bytes: 10,
        }
    }

    #[test]
    fn subscribe_channel_receives_published_updates() {
        let hub = PubSubHub::new();
        let (_id, mut receiver) = hub.subscribe_channel("task");

        hub.publish("task", sample_update(1));

        let update = receiver.try_recv().unwrap();
        assert_eq!(update.version, 1);
        assert_eq!(update.key, "task");
    }

    #[test]
    fn publish_notifies_all_subscribers_for_key() {
        let hub = PubSubHub::new();
        let first = Arc::new(RecordingSubscriber::default());
        let second = Arc::new(RecordingSubscriber::default());

        hub.subscribe_arc("task", first.clone());
        hub.subscribe_arc("task", second.clone());
        hub.publish("task", sample_update(2));

        assert_eq!(first.updates().len(), 1);
        assert_eq!(second.updates().len(), 1);
    }

    #[test]
    fn unsubscribe_removes_subscription() {
        let hub = PubSubHub::new();
        let (subscription_id, mut receiver) = hub.subscribe_channel("task");

        assert!(hub.unsubscribe(subscription_id));
        hub.publish("task", sample_update(3));

        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn publish_is_key_scoped() {
        let hub = PubSubHub::new();
        let (_id, mut task_receiver) = hub.subscribe_channel("task");
        let (_id, mut other_receiver) = hub.subscribe_channel("other");

        hub.publish("task", sample_update(4));

        assert!(task_receiver.try_recv().is_ok());
        assert!(other_receiver.try_recv().is_err());
    }

    #[test]
    fn subscriber_ids_are_unique() {
        let hub = PubSubHub::new();
        let first = hub.subscribe("task", RecordingSubscriber::default());
        let second = hub.subscribe("task", RecordingSubscriber::default());

        assert_ne!(first, second);
        assert_eq!(hub.subscriber_count("task"), 2);
    }

    #[test]
    fn trait_subscriber_is_invoked() {
        let hub = PubSubHub::new();
        let subscriber = Arc::new(RecordingSubscriber::default());

        hub.subscribe_arc("task", subscriber.clone());
        hub.publish("task", sample_update(5));

        assert_eq!(subscriber.updates()[0].version, 5);
    }
}
