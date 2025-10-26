#!/bin/bash
# Fast merge of data1.csv and data2.csv with game_id offsetting

set -e

DATA_DIR="nnue/versions/v0"
DATA1="$DATA_DIR/data1.csv"
DATA2="$DATA_DIR/data2.csv"
OUTPUT="$DATA_DIR/data.csv"
BACKUP="$DATA_DIR/data.csv.backup"

echo "=== Merging Chess Training Data ==="

# Check files exist
if [ ! -f "$DATA1" ]; then
    echo "Error: $DATA1 not found!"
    exit 1
fi

if [ ! -f "$DATA2" ]; then
    echo "Error: $DATA2 not found!"
    exit 1
fi

# Backup existing data.csv if it exists
if [ -f "$OUTPUT" ]; then
    echo "Backing up existing $OUTPUT to $BACKUP"
    mv "$OUTPUT" "$BACKUP"
fi

# Find max game_id in data1
echo "Finding max game_id in data1.csv..."
MAX_ID=$(awk -F',' '{print $4}' "$DATA1" | grep -E '^[0-9]+$' | sort -n | tail -1)
echo "Max game_id in data1: $MAX_ID"

# Calculate offset
OFFSET=$((MAX_ID + 1))
echo "Offset for data2: $OFFSET"

# Count lines
LINE_COUNT_DATA1=$(wc -l < "$DATA1")
LINE_COUNT_DATA2=$(wc -l < "$DATA2")
TOTAL_LINES=$((LINE_COUNT_DATA1 + LINE_COUNT_DATA2))
echo "Total lines to merge: $(printf "%'d" $TOTAL_LINES) ($(printf "%'d" $LINE_COUNT_DATA1) + $(printf "%'d" $LINE_COUNT_DATA2))"

# Merge files
echo ""
echo "Copying data1.csv..."
cat "$DATA1" > "$OUTPUT"
echo "  Copied $(printf "%'d" $LINE_COUNT_DATA1) lines from data1"

echo ""
echo "Copying data2.csv with offset (skipping header)..."
awk -F',' -v offset=$OFFSET '
NR == 1 {
    # Skip header row
    next
}
{
    # Offset the game_id (column 4)
    if (NF >= 4 && $4 ~ /^[0-9]+$/) {
        $4 = $4 + offset
    }
    
    # Print with comma separator
    for (i = 1; i < NF; i++) {
        printf "%s,", $i
    }
    printf "%s\n", $NF
}
' "$DATA2" >> "$OUTPUT"

FINAL_LINES=$(wc -l < "$OUTPUT")
echo "  Copied $(printf "%'d" $LINE_COUNT_DATA2) lines from data2"
echo ""
echo "Merge complete! Output written to $OUTPUT"
echo "Total lines written: $(printf "%'d" $FINAL_LINES)"

echo ""
echo "Verifying game_id sequences..."
awk -F',' '
BEGIN {
    current_game = -1
    total_games = 0
    errors = 0
    error_limit = 10
}
{
    if (NF >= 4) {
        game_id = $4
        
        # New game encountered
        if (game_id != current_game) {
            # Mark previous game as completed
            if (current_game != -1) {
                completed[current_game] = 1
            }
            
            # Check if this game was already completed
            if (game_id in completed) {
                if (errors < error_limit) {
                    printf "  ❌ Error at line %d: game_id %d appears again after completion!\n", NR, game_id
                }
                errors++
            }
            
            current_game = game_id
            total_games++
            
            if (total_games % 100000 == 0) {
                printf "  Checked %'"'"'d games...\n", total_games
            }
        }
    }
}
END {
    # Mark final game as completed
    if (current_game != -1) {
        completed[current_game] = 1
    }
    
    printf "\nTotal game sequences: %'"'"'d\n", total_games
    printf "Unique game_ids: %'"'"'d\n", length(completed)
    
    if (errors == 0) {
        print "✅ All game_ids appear in contiguous sequences!"
        exit 0
    } else {
        printf "⚠️ Found %'"'"'d game_ids that appear in non-contiguous sequences!\n", errors
        exit 1
    }
}
' "$OUTPUT"

exit $?

