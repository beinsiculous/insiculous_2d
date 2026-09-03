#!/bin/bash

# Emergency GPU Diagnostic Script for Insiculous 2D
# This script runs comprehensive diagnostics to identify GPU presentation issues

echo "🚨 INSICULOUS 2D - EMERGENCY GPU DIAGNOSTICS 🚨"
echo "=============================================="
echo ""
echo "This script will run comprehensive GPU diagnostics to identify:"
echo "  - Surface presentation issues"
echo "  - Swap chain health problems"
echo "  - Command buffer execution failures"
echo "  - Texture validation issues"
echo "  - Backend-specific problems (OpenGL ES, Vulkan, etc.)"
echo "  - Present mode compatibility"
echo ""
echo "🔍 DIAGNOSTIC STEPS:"
echo "1. Simple GPU diagnostic (basic functionality test)"
echo "2. Emergency GPU diagnostic (comprehensive analysis)"
echo "3. Sprite rendering with diagnostics (integrated test)"
echo ""

# Set up environment
export RUST_LOG=debug
export WGPU_BACKEND=primary

# Function to run diagnostic with timeout and error handling
run_diagnostic() {
    local name=$1
    local command=$2
    
    echo ""
    echo "🧪 Running: $name"
    echo "Command: $command"
    echo "----------------------------------------"
    
    # Run with timeout to prevent hanging
    timeout 30s $command
    local exit_code=$?
    
    if [ $exit_code -eq 0 ]; then
        echo "✅ $name completed successfully"
    elif [ $exit_code -eq 124 ]; then
        echo "⚠️  $name timed out after 30 seconds"
    else
        echo "❌ $name failed with exit code $exit_code"
    fi
    
    echo "----------------------------------------"
    echo ""
    
    return $exit_code
}

# Test 1: Simple GPU Diagnostic
echo "🚀 STEP 1: Simple GPU Diagnostic Test"
echo "This test checks basic GPU functionality and surface creation."
echo ""

run_diagnostic "Simple GPU Diagnostic" "cargo run --example simple_gpu_diagnostic --features tokio"

# Test 2: Emergency GPU Diagnostic
echo "🚀 STEP 2: Emergency GPU Diagnostic"
echo "This test runs comprehensive diagnostics on every level of the pipeline."
echo ""

run_diagnostic "Emergency GPU Diagnostic" "cargo run --example gpu_emergency_diagnostic --features tokio"

# Test 3: Sprite Rendering with Diagnostics
echo "🚀 STEP 3: Sprite Rendering with Integrated Diagnostics"
echo "This test combines sprite rendering with real-time pipeline inspection."
echo ""

run_diagnostic "Sprite Rendering Diagnostic" "cargo run --example sprite_rendering_diagnostic --features tokio"

# Test 4: Check for specific backend issues
echo "🚀 STEP 4: Backend-Specific Testing"
echo "Testing different WGPU backends to identify compatibility issues..."
echo ""

backends=("vulkan" "gl" "dx12" "metal")
for backend in "${backends[@]}"; do
    echo "Testing $backend backend..."
    export WGPU_BACKEND=$backend
    
    timeout 10s cargo run --example simple_gpu_diagnostic --features tokio > /tmp/diagnostic_${backend}.log 2>&1
    exit_code=$?
    
    if [ $exit_code -eq 0 ]; then
        echo "✅ $backend backend: Functional"
    else
        echo "❌ $backend backend: Issues detected"
        echo "   Check /tmp/diagnostic_${backend}.log for details"
    fi
done

# Reset to primary backend
export WGPU_BACKEND=primary

# Test 5: Validation layers
echo ""
echo "🚀 STEP 5: Validation Layer Analysis"
echo "Running with maximum validation to catch silent errors..."
echo ""

export WGPU_VALIDATION=1
export WGPU_DEBUG=1

run_diagnostic "Validation Layer Test" "timeout 15s cargo run --example simple_gpu_diagnostic --features tokio"

# Generate final report
echo ""
echo "🎯 DIAGNOSTIC SUMMARY"
echo "====================="
echo ""
echo "🔍 KEY FINDINGS TO CHECK:"
echo "1. Surface acquisition: Can the surface provide textures?"
echo "2. Command buffer submission: Are GPU commands executing?"
echo "3. Present mode compatibility: Which present modes work?"
echo "4. Backend compatibility: Are there OpenGL ES/Vulkan issues?"
echo "5. Pixel data validation: Is the correct color being rendered?"
echo "6. EGL warnings: Look for 'can present but not natively' messages"
echo ""
echo "🔧 COMMON FIXES:"
echo "- Try different present modes (FIFO, Mailbox, Immediate)"
echo "- Force Vulkan backend if OpenGL ES has issues"
echo "- Check window manager compositor settings"
echo "- Verify EGL/OpenGL ES driver compatibility"
echo "- Test with different surface formats"
echo ""
echo "📋 LOG FILES:"
echo "- Check console output above for detailed diagnostics"
echo "- Backend-specific logs: /tmp/diagnostic_*.log"
echo "- WGPU trace files: wgpu-trace/ (if enabled)"
echo ""
echo "🚨 If issues persist, report with:"
echo "- Complete diagnostic output"
echo "- GPU model and driver version"
echo "- Operating system and window manager"
echo "- Backend compatibility results"
echo ""
echo "Diagnostic script completed!"

# Clean up
unset WGPU_BACKEND
unset WGPU_VALIDATION  
unset WGPU_DEBUG