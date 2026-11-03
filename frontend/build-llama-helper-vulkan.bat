@echo off
rem Build the llama-helper sidecar with Vulkan (GPU) using Ninja; called by build-plus.sh.
rem Ninja instead of the default Visual Studio generator: MSBuild runs llama.cpp's
rem vulkan-shaders-gen ExternalProject steps out of order ("Not a file: ...cmake_install.cmake").
set "PATH=C:\Program Files (x86)\Microsoft Visual Studio\Installer;%PATH%"
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul || exit /b 1
for /d %%d in ("C:\VulkanSDK\*") do set "VULKAN_SDK=%%d"
if not defined VULKAN_SDK (echo Vulkan SDK not found in C:\VulkanSDK & exit /b 1)
set "PATH=%USERPROFILE%\.cargo\bin;C:\Program Files\CMake\bin;C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja;%VULKAN_SDK%\Bin;%PATH%"
set "CMAKE_GENERATOR=Ninja"
if defined CARGO_TARGET_DIR set "CARGO_TARGET_DIR=%CARGO_TARGET_DIR:/=\%"
cd /d "%~dp0.."
cargo build -j 4 --release -p llama-helper --features vulkan
