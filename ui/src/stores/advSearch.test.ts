import { describe, it, expect } from "vitest";
import type { SkyStringDTO } from "../api/strings";
import {
  applyFilterAndSort,
  applyAdvancedFilter,
  makeTextMatcher,
  parseFormIdInput,
  normalizeDtoFormId,
  emptyAdvSearch,
  isAdvSearchEmpty,
  AdvSearchState,
} from "./appStore";

// 辅助函数：创建测试条目
function makeItem(overrides: Partial<SkyStringDTO> = {}): SkyStringDTO {
  return {
    id: 1,
    source: "Hello World",
    translation: "你好世界",
    record_sig: "INFO",
    field_sig: "FULL",
    form_id: "0x00001234",
    status: "translated",
    list_index: 0,
    str_id: 1,
    is_vmad: false,
    ld: 0,
    ...overrides,
  };
}

function makeAdv(overrides: Partial<AdvSearchState> = {}): AdvSearchState {
  return {
    ...emptyAdvSearch(),
    ...overrides,
  };
}

describe("advSearch state helpers", () => {
  it("emptyAdvSearch returns all empty criteria with no regex and any mode", () => {
    const s = emptyAdvSearch();
    expect(isAdvSearchEmpty(s)).toBe(true);
    expect(s.useRegex).toEqual({ source: false, translated: false, edid: false, keyword: false });
    expect(s.compareMode).toBe("any");
  });

  it("isAdvSearchEmpty detects non-empty criteria", () => {
    const s = makeAdv({ criteria: { ...emptyAdvSearch().criteria, source: "foo" } });
    expect(isAdvSearchEmpty(s)).toBe(false);
  });
});

describe("makeTextMatcher", () => {
  it("plain substring match is case-insensitive", () => {
    const m = makeTextMatcher("hello", false);
    expect(m("Hello World")).toBe(true);
    expect(m("Goodbye")).toBe(false);
  });

  it("regex match is case-insensitive", () => {
    const m = makeTextMatcher("^hel.*", true);
    expect(m("Hello World")).toBe(true);
    expect(m("world")).toBe(false);
  });

  it("invalid regex matches nothing (Delphi behavior)", () => {
    const m = makeTextMatcher("[unclosed", true);
    expect(m("anything")).toBe(false);
  });
});

describe("parseFormIdInput / normalizeDtoFormId", () => {
  it("parses $ and 0x prefixed hex, normalizing case and leading zeros", () => {
    expect(parseFormIdInput("$00001234")).toBe("1234");
    expect(parseFormIdInput("0x00001234")).toBe("1234");
    expect(parseFormIdInput("0xAbCd")).toBe("abcd");
  });

  it("rejects non-hex or bare numbers", () => {
    expect(parseFormIdInput("1234")).toBeNull();
    expect(parseFormIdInput("Whiterun")).toBeNull();
    expect(parseFormIdInput("0xZZZ")).toBeNull();
  });

  it("normalizeDtoFormId strips prefix and leading zeros", () => {
    expect(normalizeDtoFormId("0x00001234")).toBe("1234");
    expect(normalizeDtoFormId("$00001234")).toBe("1234");
    expect(normalizeDtoFormId("0x001234")).toBe("1234");
  });
});

describe("applyAdvancedFilter — 每个搜索维度彼此独立", () => {
  const items: SkyStringDTO[] = [
    makeItem({ id: 1, source: "Hello World", translation: "你好世界", record_sig: "INFO", field_sig: "FULL", form_id: "0x00001234", edid: "Whiterun" }),
    makeItem({ id: 2, source: "Goodbye World", translation: "再见世界", record_sig: "QUST", field_sig: "DESC", form_id: "0x00005678", edid: "MainQuest" }),
    makeItem({ id: 3, source: "Hello Skyrim", translation: "", record_sig: "DIAL", field_sig: "NAM1", form_id: "0x00009ABC", edid: "Sovngarde" }),
  ];

  it("source 维度独立过滤", () => {
    const adv = makeAdv({ criteria: { ...emptyAdvSearch().criteria, source: "hello" } });
    const result = applyAdvancedFilter(items, adv);
    expect(result.map((i) => i.id)).toEqual([1, 3]);
  });

  it("translated 维度独立过滤", () => {
    const adv = makeAdv({ criteria: { ...emptyAdvSearch().criteria, translated: "再见" } });
    const result = applyAdvancedFilter(items, adv);
    expect(result.map((i) => i.id)).toEqual([2]);
  });

  it("多个维度 AND 组合", () => {
    const adv = makeAdv({
      criteria: { ...emptyAdvSearch().criteria, source: "world", translated: "再见" },
    });
    const result = applyAdvancedFilter(items, adv);
    expect(result.map((i) => i.id)).toEqual([2]);
  });

  it("REC 维度独立过滤", () => {
    const adv = makeAdv({ criteria: { ...emptyAdvSearch().criteria, rec: "QUST" } });
    const result = applyAdvancedFilter(items, adv);
    expect(result.map((i) => i.id)).toEqual([2]);
  });

  it("REC 匹配大小写不敏感", () => {
    const adv = makeAdv({ criteria: { ...emptyAdvSearch().criteria, rec: "qust" } });
    const result = applyAdvancedFilter(items, adv);
    expect(result.map((i) => i.id)).toEqual([2]);
  });

  it("FIELD 维度独立过滤", () => {
    const adv = makeAdv({ criteria: { ...emptyAdvSearch().criteria, field: "NAM1" } });
    const result = applyAdvancedFilter(items, adv);
    expect(result.map((i) => i.id)).toEqual([3]);
  });

  it("REC:FIELD 联合条件（单框语法）", () => {
    const adv = makeAdv({ criteria: { ...emptyAdvSearch().criteria, rec: "INFO:FULL" } });
    const result = applyAdvancedFilter(items, adv);
    expect(result.map((i) => i.id)).toEqual([1]);
  });

  it("REC 框与 FIELD 框联合（双框语法）", () => {
    const adv = makeAdv({ criteria: { ...emptyAdvSearch().criteria, rec: "QUST", field: "DESC" } });
    const result = applyAdvancedFilter(items, adv);
    expect(result.map((i) => i.id)).toEqual([2]);
  });

  it("REC 命中但 FIELD 不匹配时无结果", () => {
    const adv = makeAdv({ criteria: { ...emptyAdvSearch().criteria, rec: "QUST", field: "FULL" } });
    const result = applyAdvancedFilter(items, adv);
    expect(result).toEqual([]);
  });
});

describe("applyAdvancedFilter — EDID/FormID 维度", () => {
  const items: SkyStringDTO[] = [
    makeItem({ id: 1, source: "A", edid: "Whiterun", form_id: "0x00001234" }),
    makeItem({ id: 2, source: "B", edid: "WhiterunHold", form_id: "0x00005678" }),
    makeItem({ id: 3, source: "C", edid: null, form_id: "0x00009ABC" }),
  ];

  it("EDID 文本子串匹配（大小写不敏感）", () => {
    const adv = makeAdv({ criteria: { ...emptyAdvSearch().criteria, edid: "whiterun" } });
    const result = applyAdvancedFilter(items, adv);
    expect(result.map((i) => i.id)).toEqual([1, 2]);
  });

  it("FormID 十六进制精确匹配（$ 前缀）", () => {
    const adv = makeAdv({ criteria: { ...emptyAdvSearch().criteria, edid: "$00005678" } });
    const result = applyAdvancedFilter(items, adv);
    expect(result.map((i) => i.id)).toEqual([2]);
  });

  it("FormID 十六进制精确匹配（0x 前缀，带前导零）", () => {
    const adv = makeAdv({ criteria: { ...emptyAdvSearch().criteria, edid: "0x9ABC" } });
    const result = applyAdvancedFilter(items, adv);
    expect(result.map((i) => i.id)).toEqual([3]);
  });

  it("EDID 为 null 时文本搜索不匹配", () => {
    const adv = makeAdv({ criteria: { ...emptyAdvSearch().criteria, edid: "C" } });
    const result = applyAdvancedFilter(items, adv);
    expect(result).toEqual([]);
  });
});

describe("applyAdvancedFilter — Regex 按字段独立保存", () => {
  const items: SkyStringDTO[] = [
    makeItem({ id: 1, source: "Hello World", translation: "你好世界" }),
    makeItem({ id: 2, source: "HELLO SKYRIM", translation: "天际早安" }),
    makeItem({ id: 3, source: "Goodbye", translation: "再见" }),
  ];

  it("source 使用 regex，translated 使用普通匹配", () => {
    const adv = makeAdv({
      criteria: { ...emptyAdvSearch().criteria, source: "^hel.*", translated: "你好" },
      useRegex: { source: true, translated: false, edid: false, keyword: false },
    });
    const result = applyAdvancedFilter(items, adv);
    // id 1: source 匹配 ^hel.*，译文含“你好” → 通过
    // id 2: source 匹配 ^hel.*，但译文不含“你好” → 排除
    // id 3: source 不匹配 ^hel.* → 排除
    expect(result.map((i) => i.id)).toEqual([1]);
  });

  it("invalid regex 维度导致整体无结果", () => {
    const adv = makeAdv({
      criteria: { ...emptyAdvSearch().criteria, source: "[bad" },
      useRegex: { source: true, translated: false, edid: false, keyword: false },
    });
    const result = applyAdvancedFilter(items, adv);
    expect(result).toEqual([]);
  });
});

describe("applyAdvancedFilter — Source/Translated 比较模式", () => {
  const items: SkyStringDTO[] = [
    makeItem({ id: 1, source: "Hello", translation: "Hello" }),   // 两者匹配且相等
    makeItem({ id: 2, source: "Hello", translation: "hello!" }),  // 两者匹配但不等（regex .* 捕获不同）
    makeItem({ id: 3, source: "Hi", translation: "Hi" }),         // source 不匹配 hello
  ];

  it("eq 模式只保留 source === translated（两者都匹配时）", () => {
    const adv = makeAdv({
      criteria: { ...emptyAdvSearch().criteria, source: "hello", translated: "hello" },
      compareMode: "eq",
    });
    const result = applyAdvancedFilter(items, adv);
    expect(result.map((i) => i.id)).toEqual([1]);
  });

  it("neq 模式只保留 source !== translated（两者都匹配时）", () => {
    const adv = makeAdv({
      criteria: { ...emptyAdvSearch().criteria, source: "hello", translated: "hello" },
      compareMode: "neq",
    });
    const result = applyAdvancedFilter(items, adv);
    expect(result.map((i) => i.id)).toEqual([2]);
  });

  it("any 模式不比较（默认）：仅要求两者都匹配各自条件", () => {
    const adv = makeAdv({
      criteria: { ...emptyAdvSearch().criteria, source: "hello", translated: "hello" },
      compareMode: "any",
    });
    const result = applyAdvancedFilter(items, adv);
    expect(result.map((i) => i.id)).toEqual([1, 2]);
  });
});

describe("applyFilterAndSort — Advanced Search 集成", () => {
  const items: SkyStringDTO[] = [
    makeItem({ id: 1, source: "Hello World", translation: "你好世界", record_sig: "INFO", field_sig: "FULL", edid: "Whiterun" }),
    makeItem({ id: 2, source: "Goodbye World", translation: "再见世界", record_sig: "QUST", field_sig: "DESC", edid: "MainQuest" }),
    makeItem({ id: 3, source: "Hello Skyrim", translation: "", record_sig: "DIAL", field_sig: "NAM1", edid: "Sovngarde" }),
  ];

  it("advSearch 激活时接管文本过滤，简单 filter 被挂起", () => {
    // 简单 filter 匹配 id 1（hello）与 id 3（hello）；advSearch source=world 只匹配 1,2
    const adv = makeAdv({ criteria: { ...emptyAdvSearch().criteria, source: "world" } });
    const result = applyFilterAndSort(items, "hello", false, null, null, false, null, "id", "asc", adv);
    expect(result.map((i) => i.id)).toEqual([1, 2]);
  });

  it("advSearch 为 null 时简单搜索生效", () => {
    const result = applyFilterAndSort(items, "hello", false, null, null, false, null, "id", "asc", null);
    expect(result.map((i) => i.id)).toEqual([1, 3]);
  });

  it("advSearch 空条件时退化为仅排序（不拦截）", () => {
    const result = applyFilterAndSort(items, "", false, null, null, false, null, "id", "asc", makeAdv());
    expect(result.map((i) => i.id)).toEqual([1, 2, 3]);
  });

  it("状态过滤与 advSearch 叠加", () => {
    const adv = makeAdv({ criteria: { ...emptyAdvSearch().criteria, source: "world" } });
    const result = applyFilterAndSort(items, "", false, "translated", null, false, null, "id", "asc", adv);
    // world 匹配 1,2；translated 状态过滤掉 3（无译文）→ 1,2
    expect(result.map((i) => i.id)).toEqual([1, 2]);
  });
});
