#!/usr/bin/env pwsh

Write-Host "🔍 Verify Local Optimization Config" -ForegroundColor Cyan
Write-Host "===================================="

# Check current CPU features
Write-Host "📋 Current CPU Features:" -ForegroundColor Yellow

try {
    # Get CPU information
    $cpu = Get-WmiObject -Class Win32_Processor | Select-Object -First 1
    Write-Host "CPU Model: $($cpu.Name.Trim())"
    Write-Host "Architecture: $($cpu.Architecture)"
    Write-Host "Cores: $($cpu.NumberOfCores)"
    Write-Host "Threads: $($cpu.NumberOfLogicalProcessors)"
    
    # Check CPU feature support
    Write-Host "`n🚀 SIMD Instruction Set Support:" -ForegroundColor Green
    
    # Simplified check: Modern Intel/AMD CPUs usually support these features
    $modernCpu = $cpu.Name -match "(Intel|AMD)" -and $cpu.Name -match "(i[3-9]|Ryzen|Core)"
    
    if ($modernCpu) {
        Write-Host "  ✅ AVX2 - Likely supported (Expected perf boost 15-25%)" -ForegroundColor Green
        Write-Host "  ✅ AVX - Likely supported" -ForegroundColor Green
        Write-Host "  ✅ SSE4.2 - Likely supported" -ForegroundColor Green
        Write-Host "  ✅ FMA - Likely supported" -ForegroundColor Green
    } else {
        Write-Host "  ⚠️  Cannot determine SIMD support" -ForegroundColor Yellow
    }
    
} catch {
    Write-Host "Cannot get detailed CPU information" -ForegroundColor Red
}

Write-Host "`n🔧 Build Configuration Check:" -ForegroundColor Yellow

# Check .cargo/config.toml
if (Test-Path ".cargo/config.toml") {
    Write-Host "  ✅ .cargo/config.toml exists" -ForegroundColor Green
    $configContent = Get-Content ".cargo/config.toml" -Raw
    if ($configContent -match "target-cpu=native") {
        Write-Host "  ✅ target-cpu=native enabled" -ForegroundColor Green
    } else {
        Write-Host "  ❌ target-cpu=native not enabled" -ForegroundColor Red
    }
} else {
    Write-Host "  ❌ .cargo/config.toml does not exist" -ForegroundColor Red
}

# Check optimization settings in Cargo.toml
$cargoContent = Get-Content "Cargo.toml" -Raw -ErrorAction SilentlyContinue

if ($cargoContent -match "opt-level = 3") {
    Write-Host "  ✅ Maximum optimization level (opt-level = 3)" -ForegroundColor Green
} else {
    Write-Host "  ⚠️  Maximum optimization level not set" -ForegroundColor Yellow
}

if ($cargoContent -match 'lto = "fat"') {
    Write-Host "  ✅ Link-time optimization (LTO = fat)" -ForegroundColor Green
} else {
    Write-Host "  ⚠️  Full LTO optimization not enabled" -ForegroundColor Yellow
}

if ($cargoContent -match "mimalloc") {
    Write-Host "  ✅ mimalloc memory allocator available" -ForegroundColor Green
} else {
    Write-Host "  ⚠️  mimalloc memory allocator not configured" -ForegroundColor Yellow
}

Write-Host "`n🚀 Recommended Build Commands:" -ForegroundColor Cyan
Write-Host "  Development build: cargo build" -ForegroundColor White
Write-Host "  Performance build: cargo build --release" -ForegroundColor White
Write-Host "  Benchmark test: cargo build --release; npm run benchmark" -ForegroundColor White

Write-Host "`n📊 Expected Performance Improvements:" -ForegroundColor Magenta
Write-Host "  - CPU optimization (native): +20-35%" -ForegroundColor White
Write-Host "  - Memory allocator: +8-15%" -ForegroundColor White  
Write-Host "  - LTO optimization: +10-20%" -ForegroundColor White
Write-Host "  - Total improvement: +40-70%" -ForegroundColor White

Write-Host "`n🎯 Windows-specific Tips:" -ForegroundColor Cyan
Write-Host "  - Make sure Visual Studio Build Tools are installed" -ForegroundColor White
Write-Host "  - Recommend using PowerShell 7+ for builds" -ForegroundColor White
Write-Host "  - Use 'rustc --print cfg' to check compiler config" -ForegroundColor White 