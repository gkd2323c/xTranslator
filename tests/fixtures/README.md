# 测试数据目录

## 大型测试文件（不提交）

本目录下以下文件不会被 Git 跟踪：
- `*.esm` / `*.esp` - 游戏主文件/模组文件
- `*.bsa` / `*.ba2` - 归档文件
- `*.sst` - 大型字典文件

请配置以下环境变量：

```powershell
# Windows PowerShell
$env:XTRANSLATOR_TEST_SKYRIM_ESM = "C:\Path\To\Skyrim.esm"
$env:XTRANSLATOR_TEST_SST_PATH = "C:\Path\To\test_dicts"

# Windows CMD
set XTRANSLATOR_TEST_SKYRIM_ESM=C:\Path\To\Skyrim.esm

# Linux/macOS
export XTRANSLATOR_TEST_SKYRIM_ESM=/path/to/Skyrim.esm
```

## 小型测试文件（已提交）

- `small.esm` - 手工构造的极小 ESP 用于 CI 测试
- `test_v8.sst` - 手工构造的 SST v8 测试文件
