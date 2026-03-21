use openakta_cache::{
    CompactContext, CompactorConfig, Context, ContextCompactor, ContextEntry, ItemKind,
};

fn make_entry(index: usize, role: &str, content: String) -> ContextEntry {
    ContextEntry::new(format!("entry-{index}"), role, content)
}

fn compact(config: CompactorConfig, context: &Context) -> CompactContext {
    ContextCompactor::new(config)
        .compact(context)
        .expect("context should compact")
}

#[test]
fn full_pipeline_achieves_target_reduction_on_large_context() {
    let mut context = Context::new();

    for index in 0..90 {
        context.add_entry(
            make_entry(
                index,
                if index % 2 == 0 { "user" } else { "assistant" },
                format!(
                    "Status update {index}: acknowledged progress on routine task filler content \
                     with repeated wording for token growth and no new decisions."
                ),
            )
            .with_kind(ItemKind::Note),
        );
    }

    for index in 90..100 {
        context.add_entry(
            make_entry(
                index,
                "assistant",
                format!(
                    "Critical architecture decision {index}: required API migration, security \
                     review, and database rollback plan before release."
                ),
            )
            .with_kind(ItemKind::Decision)
            .with_priority(1.0),
        );
    }

    let compacted = compact(
        CompactorConfig {
            max_tokens: 1_200,
            ..CompactorConfig::default()
        },
        &context,
    );

    assert!(compacted.compression_ratio >= 0.60);
    assert!(compacted.compression_ratio <= 0.85);
    assert!(compacted.content.contains("Rolling summary"));
    assert!(compacted.content.contains("Critical architecture decision"));
}

#[test]
fn huge_context_respects_max_token_budget() {
    let mut context = Context::new();

    for index in 0..250 {
        context.add_entry(
            make_entry(
                index,
                "assistant",
                format!(
                    "Long execution transcript {index}: repeated implementation notes, \
                     diagnostics, and coordination language across multiple steps and files."
                ),
            )
            .with_kind(ItemKind::Turn),
        );
    }

    let compacted = compact(
        CompactorConfig {
            max_tokens: 500,
            ..CompactorConfig::default()
        },
        &context,
    );

    assert!(compacted.compacted_tokens <= 500);
}

#[test]
fn empty_context_compacts_to_empty_output() {
    let compacted = compact(CompactorConfig::default(), &Context::new());

    assert_eq!(compacted.content, "");
    assert_eq!(compacted.original_tokens, 0);
    assert_eq!(compacted.compacted_tokens, 0);
}

#[test]
fn low_importance_chatter_is_pruned_before_critical_items() {
    let mut context = Context::new();
    context.add_entry(make_entry(0, "user", "ok thanks".to_string()).with_kind(ItemKind::Note));
    context.add_entry(
        make_entry(
            1,
            "assistant",
            "Critical security decision: required rollback if API migration fails.".to_string(),
        )
        .with_kind(ItemKind::Decision)
        .with_priority(1.0),
    );

    let compacted = compact(
        CompactorConfig {
            max_tokens: 120,
            importance_threshold: 0.45,
            ..CompactorConfig::default()
        },
        &context,
    );

    assert!(compacted.content.contains("Critical security decision"));
    assert!(!compacted.content.contains("ok thanks"));
}

#[test]
fn recent_window_is_retained_verbatim() {
    let mut context = Context::new();

    for index in 0..14 {
        context.add_entry(
            make_entry(index, "assistant", format!("turn {index} detailed content"))
                .with_kind(ItemKind::Turn),
        );
    }

    let compacted = compact(CompactorConfig::default(), &context);

    assert!(compacted
        .content
        .contains("[assistant] turn 13 detailed content"));
    assert!(compacted
        .content
        .contains("[assistant] turn 4 detailed content"));
    assert!(!compacted
        .content
        .contains("[assistant] turn 0 detailed content"));
}
