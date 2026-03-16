#!/bin/bash

# Validate all business rules against schema
# Usage: ./scripts/validate-business-rules.sh

set -e

echo "╔══════════════════════════════════════════════════════════╗"
echo "║     Business Rules Validation                             ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

RULES_DIR="docs/business_rules"
ERRORS=0
WARNINGS=0

# Check if rules directory exists
if [ ! -d "$RULES_DIR" ]; then
    echo "ERROR: Business rules directory not found: $RULES_DIR"
    exit 1
fi

# Count rule files
rule_count=$(find "$RULES_DIR" -name "*.md" -type f | wc -l)
echo "Found $rule_count business rule file(s)"
echo ""

# Validate each file
for rule_file in "$RULES_DIR"/*.md; do
    if [ ! -f "$rule_file" ]; then
        continue
    fi
    
    filename=$(basename "$rule_file")
    echo "Validating $filename..."
    
    # Extract YAML frontmatter
    yaml=$(sed -n '/^---$/,/^---$/p' "$rule_file" | sed '1d;$d')
    
    # Validate rule_id pattern
    rule_id=$(echo "$yaml" | grep "^rule_id:" | sed 's/rule_id: *//' | tr -d '"' | tr -d "'")
    if [ -z "$rule_id" ]; then
        echo "  ❌ ERROR: Missing rule_id"
        ERRORS=$((ERRORS + 1))
    elif ! [[ "$rule_id" =~ ^[A-Z]{3,4}-[0-9]{3}$ ]]; then
        echo "  ❌ ERROR: Invalid rule_id format: $rule_id (expected: XXX-000)"
        ERRORS=$((ERRORS + 1))
    else
        echo "  ✓ rule_id: $rule_id"
    fi
    
    # Validate title exists
    title=$(echo "$yaml" | grep "^title:" | sed 's/title: *//' | tr -d '"' | tr -d "'")
    if [ -z "$title" ]; then
        echo "  ❌ ERROR: Missing title"
        ERRORS=$((ERRORS + 1))
    else
        echo "  ✓ title: $title"
    fi
    
    # Validate category enum
    category=$(echo "$yaml" | grep "^category:" | sed 's/category: *//' | tr -d '"' | tr -d "'")
    if [ -z "$category" ]; then
        echo "  ❌ ERROR: Missing category"
        ERRORS=$((ERRORS + 1))
    elif ! [[ "$category" =~ ^(Security|Compliance|Business|Performance|Reliability)$ ]]; then
        echo "  ❌ ERROR: Invalid category: $category"
        echo "     Expected: Security, Compliance, Business, Performance, or Reliability"
        ERRORS=$((ERRORS + 1))
    else
        echo "  ✓ category: $category"
    fi
    
    # Validate severity enum
    severity=$(echo "$yaml" | grep "^severity:" | sed 's/severity: *//' | tr -d '"' | tr -d "'")
    if [ -z "$severity" ]; then
        echo "  ❌ ERROR: Missing severity"
        ERRORS=$((ERRORS + 1))
    elif ! [[ "$severity" =~ ^(Critical|High|Medium|Low)$ ]]; then
        echo "  ❌ ERROR: Invalid severity: $severity"
        echo "     Expected: Critical, High, Medium, or Low"
        ERRORS=$((ERRORS + 1))
    else
        echo "  ✓ severity: $severity"
    fi
    
    # Validate applies_to (files must exist)
    echo "  Checking applies_to files..."
    applies_to_count=0
    while IFS= read -r line; do
        file_path=$(echo "$line" | sed 's/^  - "//' | sed 's/"$//' | tr -d "'" | xargs)
        if [ -n "$file_path" ] && [ "$file_path" != "" ]; then
            applies_to_count=$((applies_to_count + 1))
            if [ ! -f "$file_path" ]; then
                echo "  ⚠️  WARNING: File not found: $file_path"
                WARNINGS=$((WARNINGS + 1))
            else
                echo "    ✓ $file_path"
            fi
        fi
    done < <(echo "$yaml" | grep -A 100 "^applies_to:" | grep "^  - " || true)
    
    if [ $applies_to_count -eq 0 ]; then
        echo "  ❌ ERROR: applies_to is empty or missing"
        ERRORS=$((ERRORS + 1))
    fi
    
    # Validate related_rules format (if present)
    echo "  Checking related_rules..."
    while IFS= read -r line; do
        related_id=$(echo "$line" | sed 's/^  - "//' | sed 's/"$//' | tr -d "'" | xargs)
        if [ -n "$related_id" ] && [ "$related_id" != "" ]; then
            if ! [[ "$related_id" =~ ^[A-Z]{3,4}-[0-9]{3}$ ]]; then
                echo "  ⚠️  WARNING: Invalid related_rules format: $related_id"
                WARNINGS=$((WARNINGS + 1))
            else
                echo "    ✓ $related_id"
            fi
        fi
    done < <(echo "$yaml" | grep -A 100 "^related_rules:" | grep "^  - " || true)
    
    echo ""
done

# Summary
echo "╔══════════════════════════════════════════════════════════╗"
echo "║     Validation Summary                                    ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""
echo "Files validated: $rule_count"
echo "Errors: $ERRORS"
echo "Warnings: $WARNINGS"
echo ""

if [ $ERRORS -gt 0 ]; then
    echo "❌ Validation FAILED with $ERRORS error(s)"
    exit 1
elif [ $WARNINGS -gt 0 ]; then
    echo "⚠️  Validation PASSED with $WARNINGS warning(s)"
    exit 0
else
    echo "✅ All business rules validated successfully!"
    exit 0
fi
