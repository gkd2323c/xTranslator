# xTranslator 发布流程

## 当前版本
- **版本**: 1.2.0 (正式版)
- **测试**: `cargo test -p xt-core --lib` → 299 passed | `npm run test:e2e` → 64 passed
- **构建**: `./build.bat` 一键 release

## 发布步骤

### 1. 版本升级
```bash
# 更新 workspace version
# Cargo.toml: version = "0.1.0" → "0.2.0"（或 "1.0.0" 当稳定版发布时）

# 更新 src-tauri/tauri.conf.json version
# "version": "0.1.0" → "0.2.0"
```

### 2. 生成 CHANGELOG
```bash
# 从 git log 生成
git log --oneline --no-merge --pretty=format:"- %s" HEAD~N..HEAD > CHANGELOG.md

# 或使用 conventional-changelog
npx conventional-changelog -p angular -i CHANGELOG.md -s
```

### 3. 本地验证
```bash
# 后端测试
cargo test -p xt-core --lib
cargo test -p xt-core --test benchmark_ipc

# 前端测试
cd ui && npm run test

# 构建验证
cargo build -p xtranslator-tauri
cd ui && npm run build
```

### 4. 提交发布准备
```bash
git add Cargo.toml Cargo.lock src-tauri/tauri.conf.json CHANGELOG.md
git commit -m "chore: prepare v0.2.0 release"
git tag v0.2.0
```

### 5. 打包发布
```bash
# Tauri 构建（生成 .exe, .msi 等）
cargo tauri build

# 产物位置
# src-tauri/target/release/bundle/
#   - xtranslator_0.2.0_x64_en-US.msi
#   - xtranslator_0.2.0_x64_en-US.msi.bundle
#   - app-x86_64-pc-windows-msvc/xtranslator_0.2.0_x64_en-US.exe
```

### 6. GitHub Release
- 推送 tag: `git push origin v0.2.0`
- 在 GitHub Releases 创建 release
- 上传构建产物

## 版本规范
- **1.x.x**: 稳定版，API 稳定
- **语义化版本**: MAJOR.MINOR.PATCH
