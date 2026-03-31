@echo off
setlocal

set "SCRIPT_DIR=%~dp0"
set "PACKAGE_ROOT=%SCRIPT_DIR%"

if defined CC_SWITCH_WEB_DIST_DIR (
  set "DIST_DIR=%CC_SWITCH_WEB_DIST_DIR%"
) else (
  set "DIST_DIR=%PACKAGE_ROOT%dist"
)

set "BINARY_PATH=%PACKAGE_ROOT%cc-switch-web.exe"

if not exist "%DIST_DIR%" (
  echo dist 目录不存在: %DIST_DIR%
  exit /b 1
)

if not exist "%BINARY_PATH%" (
  echo 服务二进制不存在: %BINARY_PATH%
  exit /b 1
)

if not defined CC_SWITCH_WEB_HOST set "CC_SWITCH_WEB_HOST=127.0.0.1"
if not defined CC_SWITCH_WEB_PORT set "CC_SWITCH_WEB_PORT=8788"
set "CC_SWITCH_WEB_DIST_DIR=%DIST_DIR%"

echo CC Switch Web 已启动
echo 监听地址: %CC_SWITCH_WEB_HOST%:%CC_SWITCH_WEB_PORT%
echo 访问地址: http://%CC_SWITCH_WEB_HOST%:%CC_SWITCH_WEB_PORT%
echo 前端目录: %CC_SWITCH_WEB_DIST_DIR%
echo 服务二进制: %BINARY_PATH%
if /I "%CC_SWITCH_WEB_HOST%"=="0.0.0.0" (
  echo 当前绑定到 0.0.0.0，请使用服务器 IP 或本机地址访问
)
echo 按 Ctrl+C 停止服务
echo.

"%BINARY_PATH%"
exit /b %ERRORLEVEL%
