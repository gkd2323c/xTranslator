import { useState, useMemo, useEffect, useRef } from "react";
import { useAppStore } from "../stores/appStore";
import { Database, Search, ChevronDown, ChevronRight, FileText, Settings, MessageSquare, Smile } from "lucide-react";
import { useTranslation } from "react-i18next";

function useDebounce<T>(value: T, delay: number): T {
  const [debouncedValue, setDebouncedValue] = useState(value);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
    }
    timerRef.current = setTimeout(() => {
      setDebouncedValue(value);
    }, delay);

    return () => {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
    };
  }, [value, delay]);

  return debouncedValue;
}

interface CollapsibleSectionProps {
  title: string;
  icon: React.ReactNode;
  count: number;
  children: React.ReactNode;
  defaultExpanded?: boolean;
}

function CollapsibleSection({ title, icon, count, children, defaultExpanded = true }: CollapsibleSectionProps) {
  const [expanded, setExpanded] = useState(defaultExpanded);

  return (
    <div className="sidepanel-section" style={{ marginBottom: 8 }}>
      <div
        onClick={() => setExpanded(!expanded)}
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          cursor: "pointer",
          padding: "8px 0",
          borderBottom: expanded ? "1px solid var(--border-subtle)" : "none",
        }}
      >
        <h3 style={{ display: "flex", alignItems: "center", gap: 8, margin: 0 }}>
          {icon}
          {title}
          <span
            style={{
              fontSize: 11,
              background: "var(--bg-secondary)",
              padding: "2px 8px",
              borderRadius: 10,
              color: "var(--text-secondary)",
            }}
          >
            {count}
          </span>
        </h3>
        {expanded ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
      </div>
      {expanded && <div style={{ paddingTop: 8 }}>{children}</div>}
    </div>
  );
}

export function DataConfigsPanel() {
  const { t } = useTranslation();
  const dataConfigs = useAppStore((s) => s.dataConfigs);
  const espStats = useAppStore((s) => s.espStats);

  const [ctdaSearch, setCtdaSearch] = useState("");
  const [fieldSearch, setFieldSearch] = useState("");
  const [dialSearch, setDialSearch] = useState("");
  const [emoteSearch, setEmoteSearch] = useState("");

  // 防抖搜索输入，减少频繁过滤计算
  const debouncedCtdaSearch = useDebounce(ctdaSearch, 200);
  const debouncedFieldSearch = useDebounce(fieldSearch, 200);
  const debouncedDialSearch = useDebounce(dialSearch, 200);
  const debouncedEmoteSearch = useDebounce(emoteSearch, 200);

  const filteredCtda = useMemo(() => {
    if (!dataConfigs?.ctda_funcs) return [];
    if (!debouncedCtdaSearch) return dataConfigs.ctda_funcs.slice(0, 100);
    const lower = debouncedCtdaSearch.toLowerCase();
    return dataConfigs.ctda_funcs
      .filter((f) => f.name.toLowerCase().includes(lower) || f.params.toLowerCase().includes(lower))
      .slice(0, 100);
  }, [dataConfigs?.ctda_funcs, debouncedCtdaSearch]);

  const filteredFields = useMemo(() => {
    if (!dataConfigs?.field_size_ref) return [];
    const entries = Object.entries(dataConfigs.field_size_ref);
    if (!debouncedFieldSearch) return entries.slice(0, 100);
    const lower = debouncedFieldSearch.toLowerCase();
    return entries.filter(([key]) => key.toLowerCase().includes(lower)).slice(0, 100);
  }, [dataConfigs?.field_size_ref, debouncedFieldSearch]);

  const filteredDial = useMemo(() => {
    if (!dataConfigs?.dial_sub_type) return [];
    const entries = Object.entries(dataConfigs.dial_sub_type);
    if (!debouncedDialSearch) return entries.slice(0, 100);
    const lower = debouncedDialSearch.toLowerCase();
    return entries.filter(([, name]) => name.toLowerCase().includes(lower)).slice(0, 100);
  }, [dataConfigs?.dial_sub_type, debouncedDialSearch]);

  const filteredEmote = useMemo(() => {
    if (!dataConfigs?.emote_definition) return [];
    const entries = Object.entries(dataConfigs.emote_definition);
    if (!debouncedEmoteSearch) return entries.slice(0, 100);
    const lower = debouncedEmoteSearch.toLowerCase();
    return entries.filter(([, name]) => name.toLowerCase().includes(lower)).slice(0, 100);
  }, [dataConfigs?.emote_definition, debouncedEmoteSearch]);

  if (!espStats) {
    return (
      <div className="sidepanel">
        <div className="sidepanel-empty">
          <Database size={48} opacity={0.3} />
          <p>{t("dataConfigs.title")}</p>
          <p className="sidepanel-hint">{t("sidebar.loadEspToStart")}</p>
        </div>
      </div>
    );
  }

  if (!dataConfigs) {
    return (
      <div className="sidepanel">
        <div className="sidepanel-empty">
          <Database size={48} opacity={0.3} />
          <p>{t("dataConfigs.title")}</p>
          <p className="sidepanel-hint">{t("dataConfigs.subtitle")}</p>
          <p style={{ marginTop: 8, fontSize: 12, color: "var(--text-muted)" }}>
            {t("sidebar.loadEspToStart")}
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="sidepanel" style={{ overflow: "auto", height: "100%" }}>
      <CollapsibleSection
        title={t("dataConfigs.ctdaFuncs")}
        icon={<FileText size={16} />}
        count={dataConfigs.ctda_funcs.length}
      >
        <div style={{ position: "relative", marginBottom: 8 }}>
          <Search
            size={14}
            style={{ position: "absolute", left: 8, top: "50%", transform: "translateY(-50%)", opacity: 0.5 }}
          />
          <input
            type="text"
            placeholder={t("dataConfigs.searchPlaceholder")}
            value={ctdaSearch}
            onChange={(e) => setCtdaSearch(e.target.value)}
            style={{
              width: "100%",
              padding: "6px 8px 6px 28px",
              background: "var(--bg-primary)",
              border: "1px solid var(--border-subtle)",
              borderRadius: 4,
              color: "var(--text-primary)",
              fontSize: 12,
            }}
          />
        </div>
        <div style={{ maxHeight: 200, overflow: "auto" }}>
          {filteredCtda.map((func) => (
            <div
              key={func.id}
              style={{
                padding: "4px 0",
                borderBottom: "1px solid var(--border-subtle)",
                fontSize: 11,
              }}
            >
              <span
                style={{
                  fontFamily: "'JetBrains Mono', monospace",
                  color: "var(--accent-cyan)",
                  marginRight: 8,
                }}
              >
                0x{func.id.toString(16).toUpperCase().padStart(4, "0")}
              </span>
              <span style={{ color: "var(--text-primary)" }}>{func.name}</span>
              {func.params && (
                <span style={{ color: "var(--text-muted)", marginLeft: 4 }}>{func.params}</span>
              )}
            </div>
          ))}
        </div>
        {dataConfigs.ctda_funcs.length > 100 && (
          <p style={{ fontSize: 10, color: "var(--text-muted)", marginTop: 4 }}>
            Showing 100 of {dataConfigs.ctda_funcs.length} (use search)
          </p>
        )}
      </CollapsibleSection>

      <CollapsibleSection
        title={t("dataConfigs.fieldSizes")}
        icon={<Settings size={16} />}
        count={Object.keys(dataConfigs.field_size_ref).length}
      >
        <div style={{ position: "relative", marginBottom: 8 }}>
          <Search
            size={14}
            style={{ position: "absolute", left: 8, top: "50%", transform: "translateY(-50%)", opacity: 0.5 }}
          />
          <input
            type="text"
            placeholder={t("dataConfigs.searchPlaceholder")}
            value={fieldSearch}
            onChange={(e) => setFieldSearch(e.target.value)}
            style={{
              width: "100%",
              padding: "6px 8px 6px 28px",
              background: "var(--bg-primary)",
              border: "1px solid var(--border-subtle)",
              borderRadius: 4,
              color: "var(--text-primary)",
              fontSize: 12,
            }}
          />
        </div>
        <div style={{ maxHeight: 200, overflow: "auto" }}>
          {filteredFields.map(([key, info]) => (
            <div
              key={key}
              style={{
                padding: "4px 0",
                borderBottom: "1px solid var(--border-subtle)",
                fontSize: 11,
                display: "flex",
                justifyContent: "space-between",
              }}
            >
              <span>
                <span
                  style={{
                    fontFamily: "'JetBrains Mono', monospace",
                    color: "var(--accent-gold)",
                  }}
                >
                  {key}
                </span>
              </span>
              <span>
                <span style={{ color: "var(--text-secondary)" }}>{info.max_size}</span>
                {info.can_wrap && (
                  <span
                    style={{
                      fontSize: 9,
                      marginLeft: 4,
                      color: "var(--success)",
                      background: "rgba(0,255,136,0.1)",
                      padding: "1px 4px",
                      borderRadius: 3,
                    }}
                  >
                    {t("dataConfigs.canWrap")}
                  </span>
                )}
              </span>
            </div>
          ))}
        </div>
        {Object.keys(dataConfigs.field_size_ref).length > 100 && (
          <p style={{ fontSize: 10, color: "var(--text-muted)", marginTop: 4 }}>
            Showing 100 of {Object.keys(dataConfigs.field_size_ref).length} (use search)
          </p>
        )}
      </CollapsibleSection>

      <CollapsibleSection
        title={t("dataConfigs.dialSubTypes")}
        icon={<MessageSquare size={16} />}
        count={Object.keys(dataConfigs.dial_sub_type).length}
        defaultExpanded={false}
      >
        <div style={{ position: "relative", marginBottom: 8 }}>
          <Search
            size={14}
            style={{ position: "absolute", left: 8, top: "50%", transform: "translateY(-50%)", opacity: 0.5 }}
          />
          <input
            type="text"
            placeholder={t("dataConfigs.searchPlaceholder")}
            value={dialSearch}
            onChange={(e) => setDialSearch(e.target.value)}
            style={{
              width: "100%",
              padding: "6px 8px 6px 28px",
              background: "var(--bg-primary)",
              border: "1px solid var(--border-subtle)",
              borderRadius: 4,
              color: "var(--text-primary)",
              fontSize: 12,
            }}
          />
        </div>
        <div style={{ maxHeight: 200, overflow: "auto" }}>
          {filteredDial.map(([id, name]) => (
            <div
              key={id}
              style={{
                padding: "4px 0",
                borderBottom: "1px solid var(--border-subtle)",
                fontSize: 11,
              }}
            >
              <span
                style={{
                  fontFamily: "'JetBrains Mono', monospace",
                  color: "var(--accent-cyan)",
                  marginRight: 8,
                }}
              >
                {id}
              </span>
              <span style={{ color: "var(--text-primary)" }}>{name}</span>
            </div>
          ))}
        </div>
      </CollapsibleSection>

      <CollapsibleSection
        title={t("dataConfigs.emoteDefs")}
        icon={<Smile size={16} />}
        count={Object.keys(dataConfigs.emote_definition).length}
        defaultExpanded={false}
      >
        <div style={{ position: "relative", marginBottom: 8 }}>
          <Search
            size={14}
            style={{ position: "absolute", left: 8, top: "50%", transform: "translateY(-50%)", opacity: 0.5 }}
          />
          <input
            type="text"
            placeholder={t("dataConfigs.searchPlaceholder")}
            value={emoteSearch}
            onChange={(e) => setEmoteSearch(e.target.value)}
            style={{
              width: "100%",
              padding: "6px 8px 6px 28px",
              background: "var(--bg-primary)",
              border: "1px solid var(--border-subtle)",
              borderRadius: 4,
              color: "var(--text-primary)",
              fontSize: 12,
            }}
          />
        </div>
        <div style={{ maxHeight: 200, overflow: "auto" }}>
          {filteredEmote.map(([id, name]) => (
            <div
              key={id}
              style={{
                padding: "4px 0",
                borderBottom: "1px solid var(--border-subtle)",
                fontSize: 11,
              }}
            >
              <span
                style={{
                  fontFamily: "'JetBrains Mono', monospace",
                  color: "var(--accent-cyan)",
                  marginRight: 8,
                }}
              >
                {id}
              </span>
              <span style={{ color: "var(--text-primary)" }}>{name}</span>
            </div>
          ))}
        </div>
      </CollapsibleSection>
    </div>
  );
}
