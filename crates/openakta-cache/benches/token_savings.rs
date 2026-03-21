//! Token Savings Benchmarks for Phase 2 Optimizations
//!
//! This benchmark suite measures the token savings achieved by:
//! - Code minification (Sprint 3)
//! - Diff-based communication (Sprint 2)
//! - Combined optimizations

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use openakta_cache::{CodeMinifier, Schema, ToonSerializer, UnifiedDiff};
use openakta_proto::prost::Message as ProstMessage;
use openakta_proto::{ContextPack, Message, MessageType};

/// Sample Rust code for benchmarking
const SAMPLE_RUST_CODE: &str = r#"
/// Authentication module for user login and session management
/// 
/// This module provides comprehensive authentication functionality
/// including password-based login, token-based sessions, and OAuth integration.

use std::time::{Duration, Instant};
use crate::error::AuthError;
use crate::models::{User, Token, Session};

/// Authenticate a user with username and password
/// 
/// # Arguments
/// * `username` - The user's username or email
/// * `password` - The user's plain text password
/// 
/// # Returns
/// * `Ok(Token)` - Authentication successful with session token
/// * `Err(AuthError)` - Authentication failed
/// 
/// # Example
/// ```
/// let token = authenticate_user("john@example.com", "secret123")?;
/// println!("Token: {}", token);
/// ```
pub fn authenticate_user_with_credentials(
    username: &str, 
    password: &str
) -> Result<Token, AuthError> {
    // Validate input parameters
    if username.is_empty() || password.is_empty() {
        return Err(AuthError::InvalidCredentials);
    }

    // Hash the password with salt
    let salt = get_salt_for_username(username)?;
    let salted_password = format!("{}:{}", salt, password);
    let hashed_password = hash_password(&salted_password)?;

    // Query the database for user
    let user = find_user_by_username(username)?
        .ok_or(AuthError::UserNotFound)?;

    // Compare password hashes
    if user.password_hash != hashed_password {
        // Log failed attempt for security monitoring
        log_failed_login_attempt(username, &user.id)?;
        return Err(AuthError::InvalidCredentials);
    }

    // Check if account is locked
    if user.is_locked {
        return Err(AuthError::AccountLocked);
    }

    // Check if account is active
    if !user.is_active {
        return Err(AuthError::AccountInactive);
    }

    // Generate session token
    let token = Token::new(
        user.id.clone(),
        user.role.clone(),
        Duration::from_hours(24),
    );

    // Store token in session cache
    cache_session_token(&token)?;

    // Update last login timestamp
    update_last_login_timestamp(&user.id)?;

    // Clear failed login attempts
    clear_failed_login_attempts(&user.id)?;

    // Log successful login for audit trail
    log_successful_login(&user.id, username)?;

    Ok(token)
}

/// Calculate monthly revenue metrics for business dashboard
/// 
/// This function aggregates all transactions for the given period
/// and calculates comprehensive revenue metrics including:
/// - Total revenue
/// - Average transaction value
/// - Transaction count
/// - Growth rate compared to previous period
/// 
/// # Arguments
/// * `year` - The year to calculate metrics for
/// * `month` - The month to calculate metrics for (1-12)
/// 
/// # Returns
/// * `Ok(RevenueMetrics)` - Calculated metrics
/// * `Err(MetricsError)` - Calculation failed
pub fn calculate_monthly_revenue_metrics_for_dashboard(
    year: i32,
    month: u32
) -> Result<RevenueMetrics, MetricsError> {
    // Validate month parameter
    if month < 1 || month > 12 {
        return Err(MetricsError::InvalidMonth(month));
    }

    // Fetch all transactions for the period
    let transactions = fetch_all_transactions_for_period(year, month)?;

    // Filter completed transactions only
    let completed_transactions: Vec<_> = transactions
        .iter()
        .filter(|t| t.status == TransactionStatus::Completed)
        .collect();

    // Calculate total revenue
    let total_revenue = completed_transactions
        .iter()
        .map(|t| t.amount)
        .sum::<Decimal>();

    // Calculate average transaction value
    let average_transaction_value = if !completed_transactions.is_empty() {
        total_revenue / completed_transactions.len() as i32
    } else {
        Decimal::ZERO
    };

    // Calculate median transaction value
    let median_transaction_value = calculate_median_transaction_value(&completed_transactions);

    // Calculate growth rate compared to previous month
    let previous_month = if month == 1 { 12 } else { month - 1 };
    let previous_year = if month == 1 { year - 1 } else { year };
    let previous_metrics = calculate_monthly_revenue_metrics_for_dashboard(previous_year, previous_month)?;
    
    let growth_rate = if previous_metrics.total_revenue != Decimal::ZERO {
        (total_revenue - previous_metrics.total_revenue) / previous_metrics.total_revenue
    } else {
        Decimal::ZERO
    };

    // Calculate refund rate
    let refunded_transactions: Vec<_> = transactions
        .iter()
        .filter(|t| t.status == TransactionStatus::Refunded)
        .collect();
    
    let refund_rate = if !transactions.is_empty() {
        refunded_transactions.len() as f64 / transactions.len() as f64
    } else {
        0.0
    };

    Ok(RevenueMetrics {
        total_revenue,
        average_transaction_value,
        median_transaction_value,
        transaction_count: completed_transactions.len(),
        growth_rate,
        refund_rate,
        period: RevenuePeriod { year, month },
    })
}

/// Process user permission checks for resource access
/// 
/// This function implements role-based access control (RBAC)
/// with support for hierarchical permissions and resource ownership.
/// 
/// # Arguments
/// * `user` - The user requesting access
/// * `resource` - The resource being accessed
/// * `action` - The action being performed (read, write, delete)
/// 
/// # Returns
/// * `true` - Access granted
/// * `false` - Access denied
pub fn check_user_permission_for_resource_access(
    user: &User,
    resource: &Resource,
    action: &Action
) -> bool {
    // Check if user is authenticated
    if !user.is_authenticated {
        return false;
    }

    // Check if user account is active
    if !user.account_status.is_active() {
        return false;
    }

    // Check for admin override
    if user.has_role(Role::Admin) {
        return true;
    }

    // Check resource ownership
    if resource.owner_id == user.id {
        return true;
    }

    // Check explicit permissions
    let required_permission = get_required_permission_for_action(action);
    if user.has_permission(required_permission) {
        return true;
    }

    // Check role-based permissions
    for role in &user.roles {
        if role.has_permission(required_permission, &resource.type_) {
            return true;
        }
    }

    // Check group-based permissions
    for group in &user.groups {
        if group.has_permission(required_permission, &resource.type_) {
            return true;
        }
    }

    // Check inherited permissions from parent resources
    if let Some(parent) = &resource.parent {
        return check_user_permission_for_resource_access(user, parent, action);
    }

    // Access denied by default
    false
}
"#;

/// Modified version with small changes
const MODIFIED_RUST_CODE: &str = r#"
pub fn authenticate_user_with_credentials(
    username: &str, 
    password: &str
) -> Result<Token, AuthError> {
    if username.is_empty() || password.is_empty() {
        return Err(AuthError::InvalidCredentials);
    }

    let salt = get_salt_for_username(username)?;
    let salted_password = format!("{}:{}", salt, password);
    let hashed_password = hash_password(&salted_password)?;

    let user = find_user_by_username(username)?
        .ok_or(AuthError::UserNotFound)?;

    if user.password_hash != hashed_password {
        log_failed_login_attempt(username, &user.id)?;
        return Err(AuthError::InvalidCredentials);
    }

    if user.is_locked {
        return Err(AuthError::AccountLocked);
    }

    if !user.is_active {
        return Err(AuthError::AccountInactive);
    }

    let token = Token::new(user.id.clone(), user.role.clone(), Duration::from_hours(24));
    cache_session_token(&token)?;
    update_last_login_timestamp(&user.id)?;
    clear_failed_login_attempts(&user.id)?;
    log_successful_login(&user.id, username)?;

    Ok(token)
}
"#;

fn benchmark_code_minification_savings(c: &mut Criterion) {
    let minifier = CodeMinifier::new();

    c.bench_function("code_minification_savings", |b| {
        b.iter(|| {
            let minified = minifier
                .minify(black_box(SAMPLE_RUST_CODE), "rust")
                .unwrap();
            let savings = minified.savings_percentage;
            assert!(savings > 15.0, "Expected >15% savings, got {:.1}%", savings);
            black_box(minified);
        })
    });
}

fn benchmark_diff_savings(c: &mut Criterion) {
    c.bench_function("diff_savings_small_change", |b| {
        b.iter(|| {
            let diff = UnifiedDiff::generate(
                black_box(SAMPLE_RUST_CODE),
                black_box(MODIFIED_RUST_CODE),
                "old.rs",
                "new.rs",
            );
            let original_tokens = SAMPLE_RUST_CODE.len() / 4;
            let diff_tokens = diff.to_string().len() / 4;
            let savings = ((original_tokens - diff_tokens) as f32 / original_tokens as f32) * 100.0;
            assert!(
                savings > 50.0,
                "Expected >50% diff savings, got {:.1}%",
                savings
            );
            black_box(diff);
        })
    });
}

fn benchmark_combined_optimization(c: &mut Criterion) {
    let minifier = CodeMinifier::new();

    c.bench_function("combined_minification_plus_diff", |b| {
        b.iter(|| {
            // Step 1: Minify original code
            let minified = minifier
                .minify(black_box(SAMPLE_RUST_CODE), "rust")
                .unwrap();

            // Step 2: Generate diff for change
            let diff = UnifiedDiff::generate(
                black_box(SAMPLE_RUST_CODE),
                black_box(MODIFIED_RUST_CODE),
                "old.rs",
                "new.rs",
            );

            // Step 3: Calculate combined savings
            let original_tokens = SAMPLE_RUST_CODE.len() / 4 * 2; // Sending full code twice
            let optimized_tokens = minified.content.len() / 4 + diff.to_string().len() / 4;
            let savings =
                ((original_tokens - optimized_tokens) as f32 / original_tokens as f32) * 100.0;

            assert!(
                savings > 40.0,
                "Expected >40% combined savings, got {:.1}%",
                savings
            );
            black_box((minified, diff));
        })
    });
}

fn benchmark_minification_by_language(c: &mut Criterion) {
    let minifier = CodeMinifier::new();

    let languages = vec![
        ("rust", SAMPLE_RUST_CODE),
        (
            "typescript",
            r#"
// TypeScript authentication service
// Handles user login and session management

interface AuthenticationResult {
    userId: string;
    token: string;
    expiresIn: number;
}

interface UserCredentials {
    username: string;
    password: string;
}

/**
 * Authenticate user with credentials
 * @param credentials User login credentials
 * @returns Authentication result with token
 */
export async function authenticateUserWithCredentials(
    credentials: UserCredentials
): Promise<AuthenticationResult> {
    // Validate input
    if (!credentials.username || !credentials.password) {
        throw new Error('Invalid credentials');
    }

    // Hash password
    const saltedPassword = `${credentials.username}:${credentials.password}`;
    const hashedPassword = await hashPassword(saltedPassword);

    // Find user
    const user = await findUserByUsername(credentials.username);
    if (!user) {
        throw new Error('User not found');
    }

    // Verify password
    if (user.passwordHash !== hashedPassword) {
        await logFailedLoginAttempt(credentials.username);
        throw new Error('Invalid credentials');
    }

    // Generate token
    const token = generateJwtToken(user.id, user.role);
    
    return {
        userId: user.id,
        token: token,
        expiresIn: 86400, // 24 hours
    };
}
"#,
        ),
        (
            "python",
            r#"
# Python authentication module
# Provides user authentication and session management

from typing import Optional, Dict, Any
from datetime import datetime, timedelta
import hashlib

class AuthenticationError(Exception):
    """Base exception for authentication errors"""
    pass

class InvalidCredentialsError(AuthenticationError):
    """Raised when credentials are invalid"""
    pass

class UserNotFoundError(AuthenticationError):
    """Raised when user is not found"""
    pass

def authenticate_user_with_credentials(
    username: str,
    password: str
) -> Dict[str, Any]:
    """
    Authenticate a user with username and password.
    
    Args:
        username: The user's username or email
        password: The user's plain text password
        
    Returns:
        Dictionary with user_id and token
        
    Raises:
        InvalidCredentialsError: When credentials are invalid
        UserNotFoundError: When user doesn't exist
    """
    # Validate input
    if not username or not password:
        raise InvalidCredentialsError("Username and password required")
    
    # Hash password with salt
    salt = get_salt_for_username(username)
    salted_password = f"{salt}:{password}"
    hashed_password = hashlib.sha256(salted_password.encode()).hexdigest()
    
    # Find user in database
    user = find_user_by_username(username)
    if not user:
        raise UserNotFoundError(f"User {username} not found")
    
    # Verify password
    if user['password_hash'] != hashed_password:
        log_failed_login_attempt(username, user['id'])
        raise InvalidCredentialsError("Invalid password")
    
    # Check account status
    if user.get('is_locked'):
        raise AuthenticationError("Account is locked")
    
    # Generate session token
    token = generate_session_token(user['id'], user['role'])
    
    # Update last login
    update_last_login_timestamp(user['id'])
    
    return {
        'user_id': user['id'],
        'token': token,
        'expires_at': datetime.now() + timedelta(hours=24)
    }
"#,
        ),
    ];

    let mut group = c.benchmark_group("minification_by_language");

    for (lang, code) in languages {
        group.bench_with_input(BenchmarkId::from_parameter(lang), code, |b, code| {
            b.iter(|| {
                let minified = minifier.minify(black_box(code), lang).unwrap();
                let savings = minified.savings_percentage;
                black_box((minified, savings));
            })
        });
    }

    group.finish();
}

fn benchmark_identifier_compression(c: &mut Criterion) {
    let code_with_long_identifiers = r#"
pub fn calculateMonthlyRevenueMetricsForDashboard(
    totalRevenueAmount: number,
    previousMonthRevenue: number,
    transactionCount: number
) -> RevenueMetricsObject {
    const averageTransactionValue = totalRevenueAmount / transactionCount;
    const revenueGrowthRate = (totalRevenueAmount - previousMonthRevenue) / previousMonthRevenue;
    const projectedAnnualRevenue = totalRevenueAmount * 12;
    
    return {
        totalRevenue: totalRevenueAmount,
        averageTransaction: averageTransactionValue,
        growthRate: revenueGrowthRate,
        projectedAnnual: projectedAnnualRevenue,
    };
}
"#;

    let minifier = CodeMinifier::new();

    c.bench_function("identifier_compression_savings", |b| {
        b.iter(|| {
            let minified = minifier
                .minify(black_box(code_with_long_identifiers), "typescript")
                .unwrap();
            assert!(
                !minified.identifier_map.is_empty(),
                "Expected identifiers to be compressed"
            );
            black_box(minified);
        })
    });
}

fn benchmark_comment_stripping(c: &mut Criterion) {
    let code_with_comments = r#"
/// This is a documentation comment for the function
/// It explains what the function does in detail
/// 
/// # Arguments
/// * `param1` - First parameter description
/// * `param2` - Second parameter description
/// 
/// # Returns
/// The result of the operation
/// 
/// # Example
/// ```
/// let result = my_function(1, 2);
/// ```
pub fn my_function_with_documentation(
    param1: i32,  // This is an inline comment for param1
    param2: i32,  // This is an inline comment for param2
) -> i32 {
    // This is a block comment
    // explaining the implementation
    // in multiple lines
    
    let result = param1 + param2;  // Calculate sum
    
    /* 
     * This is a multi-line block comment
     * that spans several lines
     * for detailed explanation
     */
    
    result  // Return the result
}
"#;

    let minifier = CodeMinifier::new();

    c.bench_function("comment_stripping_savings", |b| {
        b.iter(|| {
            let minified = minifier
                .minify(black_box(code_with_comments), "rust")
                .unwrap();
            // Verify comments are stripped
            assert!(!minified.content.contains("/// This is a documentation"));
            assert!(!minified.content.contains("// This is an inline comment"));
            black_box(minified);
        })
    });
}

fn benchmark_model_boundary_compaction(c: &mut Criterion) {
    let context_json = serde_json::json!({
        "task_id": "task-1",
        "target_files": ["src/auth.rs", "src/db.rs"],
        "symbols": ["auth::login", "db::query"],
        "diagnostics": ["budget_exhausted:false", "cached:true"],
        "summary": "Return unified diff only"
    })
    .to_string();
    let mut schema = Schema::new();
    for field in [
        "task_id",
        "target_files",
        "symbols",
        "diagnostics",
        "summary",
    ] {
        schema.add_field(field);
    }
    let serializer = ToonSerializer::new(schema);

    c.bench_function("json_vs_toon_payload", |b| {
        b.iter(|| {
            let toon = serializer.encode(black_box(&context_json)).unwrap();
            black_box((context_json.len(), toon.len()));
        })
    });

    let proto_message = Message {
        id: "msg-1".to_string(),
        sender_id: "coordinator".to_string(),
        recipient_id: "worker".to_string(),
        message_type: MessageType::ContextPack as i32,
        content: context_json.clone(),
        timestamp: None,
        patch: None,
        patch_receipt: None,
        context_pack: Some(ContextPack {
            id: "ctx-1".to_string(),
            task_id: "task-1".to_string(),
            target_files: vec!["src/auth.rs".to_string()],
            symbols: vec!["auth::login".to_string()],
            spans: vec![],
            toon_payload: String::new(),
            base_revision: "rev-1".to_string(),
            ..Default::default()
        }),
        validation_result: None,
        task_assignment: None,
        progress_update: None,
        result_submission: None,
        blocker_alert: None,
        workflow_transition: None,
        human_question: None,
        human_answer: None,
    };

    c.bench_function("string_vs_protobuf_transport", |b| {
        b.iter(|| {
            let mut bytes = Vec::new();
            proto_message.encode(&mut bytes).unwrap();
            black_box((context_json.len(), bytes.len()));
        })
    });
}

criterion_group!(
    benches,
    benchmark_code_minification_savings,
    benchmark_diff_savings,
    benchmark_combined_optimization,
    benchmark_minification_by_language,
    benchmark_identifier_compression,
    benchmark_comment_stripping,
    benchmark_model_boundary_compaction,
);
criterion_main!(benches);
