use axora_cache::blackboard::v2::BlackboardV2;
use serde_json::json;
use std::sync::Arc;

#[test]
fn tracks_versions_across_writes() {
    let blackboard = BlackboardV2::new();

    let first = blackboard.set("task", json!({"status": "queued"})).unwrap();
    let second = blackboard
        .set("task", json!({"status": "running"}))
        .unwrap();

    assert_eq!(first, 1);
    assert_eq!(second, 2);
    assert_eq!(blackboard.current_version(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_updates_do_not_conflict() {
    let blackboard = Arc::new(BlackboardV2::new());
    let mut handles = Vec::new();

    for index in 0..100_u64 {
        let blackboard = Arc::clone(&blackboard);
        handles.push(tokio::spawn(async move {
            blackboard
                .set("counter", json!({"writer": index, "value": index}))
                .unwrap()
        }));
    }

    let mut versions = Vec::new();
    for handle in handles {
        versions.push(handle.await.unwrap());
    }

    versions.sort_unstable();
    assert_eq!(versions.first().copied(), Some(1));
    assert_eq!(versions.last().copied(), Some(100));
    assert_eq!(blackboard.current_version(), 100);
}

#[test]
fn get_with_version_does_not_return_stale_values() {
    let blackboard = BlackboardV2::new();
    blackboard.set("task", json!({"status": "queued"})).unwrap();
    blackboard
        .set("task", json!({"status": "running"}))
        .unwrap();

    let update = blackboard.get_with_version("task", 1).unwrap();

    assert_eq!(update.version, 2);
    assert_eq!(update.new_value, json!({"status": "running"}));
}

#[test]
fn all_subscribers_are_notified() {
    let blackboard = BlackboardV2::new();
    let (_first_id, mut first) = blackboard.subscribe_channel("task");
    let (_second_id, mut second) = blackboard.subscribe_channel("task");

    blackboard.set("task", json!({"status": "done"})).unwrap();

    assert_eq!(first.try_recv().unwrap().version, 1);
    assert_eq!(second.try_recv().unwrap().version, 1);
}

#[test]
fn unsubscribe_stops_future_notifications() {
    let blackboard = BlackboardV2::new();
    let (subscription_id, mut receiver) = blackboard.subscribe_channel("task");

    assert!(blackboard.unsubscribe(subscription_id));
    blackboard.set("task", json!({"status": "done"})).unwrap();

    assert!(receiver.try_recv().is_err());
}

#[test]
fn diff_push_reduces_update_size_for_large_documents() {
    let blackboard = BlackboardV2::new();
    blackboard
        .set(
            "doc",
            json!({
                "title": "Architecture Ledger",
                "body": "x".repeat(8_000),
                "status": "draft",
                "owner": "agent-a"
            }),
        )
        .unwrap();

    let update = blackboard
        .get_with_version(
            "doc",
            blackboard
                .publish(
                    "doc",
                    json!({
                        "title": "Architecture Ledger",
                        "body": "x".repeat(8_000),
                        "status": "published",
                        "owner": "agent-a"
                    }),
                )
                .unwrap()
                - 1,
        )
        .unwrap();

    assert!(update.size_reduction() > 0.8);
}
