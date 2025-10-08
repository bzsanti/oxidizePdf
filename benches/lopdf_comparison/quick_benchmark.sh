#!/bin/bash
# Quick benchmark runner - measures creation performance without full build

echo "🔥 Quick Benchmark: oxidize-pdf vs lopdf"
echo "========================================"
echo ""

# Create results directory
mkdir -p results

# Build just the creation benchmark (with progress)
echo "📦 Building benchmark (this may take a few minutes)..."
cargo build --release --bin benchmark_creation 2>&1 | grep -E "(Compiling|Finished|error)" &
BUILD_PID=$!

# Wait with timeout
TIMEOUT=300
ELAPSED=0
while kill -0 $BUILD_PID 2>/dev/null; do
    sleep 5
    ELAPSED=$((ELAPSED + 5))
    if [ $ELAPSED -ge $TIMEOUT ]; then
        echo "⚠️  Build timeout after ${TIMEOUT}s"
        kill $BUILD_PID 2>/dev/null
        exit 1
    fi
    echo -n "."
done

wait $BUILD_PID
BUILD_STATUS=$?

if [ $BUILD_STATUS -ne 0 ]; then
    echo ""
    echo "❌ Build failed. Running cargo check for details..."
    cargo check 2>&1 | tail -20
    exit 1
fi

echo ""
echo "✅ Build complete!"
echo ""

# Run benchmark
echo "🏃 Running creation benchmark..."
./target/release/benchmark_creation

echo ""
echo "📁 Results saved to: benches/lopdf_comparison/results/"
