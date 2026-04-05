# Web 端对齐记录（2026-04）

本轮对齐目标：把桌面端已新增、而 Web 端缺失的关键能力补齐到当前 Web 分支。

本轮已落地：

- 供应商模型拉取
  - Claude / Codex / Gemini / OpenClaw 表单支持直接拉取模型列表
- 官方订阅额度展示
  - Claude / Codex / Gemini 卡片底部展示官方订阅额度信息
- 环境变量冲突检测
  - 启动与切换应用时检测冲突
  - 提供批量删除与备份恢复接口
- Deep Link 导入
  - 支持页面 URL 参数 `?deeplink=...`
  - 支持在设置页手动粘贴 `ccswitch://...` 导入
  - 覆盖 provider / prompt / mcp / skill repo 四类资源
- About 更新入口
  - Web 端不做自动下载与安装
  - 改为直接打开 GitHub Releases latest

实现约束：

- 保持单 Agent 实现
- 未自动执行 build、全量测试、重型验证
- 文档与 README 三语同步更新

轻量自检：

- `git diff --check`
- 三语 i18n JSON 解析校验
