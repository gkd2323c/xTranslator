# FUZ 音频容器格式

> 来源：T18 FUZ 音频映射实现中的逆向分析
> 参考：Delphi `TESVT_Fuz.pas` 的 `getFuzFromBuffer` 函数

---

## 二进制格式

```
FUZ Container Layout
┌─────────────────────────────────────┐
│ Magic:    [u8; 4] = b"FUZE"        │
│ Unknown:  u32                       │  ← 可能为 LIP 格式版本标记
│ LipSize:  u32                       │  ← LIP 唇形同步数据大小
├─────────────────────────────────────┤
│ LIP Data: bytes[LipSize]            │  ← 可跳过
├─────────────────────────────────────┤
│ WAV Data: remaining bytes           │  ← 标准 RIFF/WAV 格式
└─────────────────────────────────────┘
```

## WAV 时长计算

```
Duration(秒) = data_chunk_size / byte_rate

其中：
  sample_rate: WAV fmt chunk offset 24 (u32)
  byte_rate:   WAV fmt chunk offset 28 (u32)
  data_size:   查找 "data" chunk 后的 u32 值
```

## 文件关联

- FUZ 文件名格式：`<VoiceTypeID>_<ResponseID>_<Index>.fuz`
- 关联方式：解析文件名中的 hex ResponseID 与已加载的 `SkyString.str_id` 匹配
- Voice 目录递归扫描，支持深层嵌套目录结构

## Delphi 代码参考

`TESVT_Fuz.pas` 的核心类 `tFuz` 负责：
1. 扫描 Voice 目录收集所有 FUZ 文件（`tfuzExport` 列表）
2. 通过 `rdialInfo` 记录的 `r.hVoiceID` 和 `r.rID` 匹配对话
3. `getFuzFromBuffer` 提取 LIP 后的 WAV 数据

我们的实现简化了 Delphi 的多 BSA/松散文件 fallback 逻辑，专注于单目录扫描和音频播放。
