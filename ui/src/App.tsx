import { useEffect } from "react";
import { Toaster } from "react-hot-toast";
import { useAppStore } from "./stores/appStore";
import { MenuBar } from "./components/MenuBar";
import { SidePanel } from "./components/SidePanel";
import { StringTable } from "./components/StringTable";
import { EditorPanel } from "./components/EditorPanel";
import "./App.css";

function App() {
  const setSelectedById = useAppStore((s) => s.setSelectedById);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setSelectedById(null);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [setSelectedById]);

  return (
    <div className="app">
      <Toaster position="top-right" />
      <MenuBar />
      <div className="app-body">
        <aside className="app-sidebar">
          <SidePanel />
        </aside>
        <main className="app-main">
          <div className="app-table-area">
            <StringTable />
          </div>
          <div className="app-editor-area">
            <EditorPanel />
          </div>
        </main>
      </div>
    </div>
  );
}

export default App;
