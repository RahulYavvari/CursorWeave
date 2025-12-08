// src/components/ThemesPanel.tsx
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export default function ThemesPanel() {
  const [themes, setThemes] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function applyTheme(name: string) {
    const root = await invoke<string>("get_themes_root");
    const themePath = `${root}\\${name}`;

    console.log("themePath sent:", themePath);

    try {
      const res = await invoke<string>("apply_theme", { themeDir: themePath });
      console.log("apply theme:", res);
    } catch (err) {
      console.error("apply error:", err);
    }
  }



  async function fetchThemes() {
    setLoading(true);
    setError(null);
    try {
      // call the Rust command
      const t = await invoke<string[]>("list_themes");
      setThemes(t || []);
      console.log("themes ->", t);
    } catch (err) {
      console.error("list_themes error", err);
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    fetchThemes();
  }, []);


  return (
    <div style={{ padding: 16 }}>
      <h3>CursorWeave — Themes</h3>
      <button onClick={fetchThemes} disabled={loading}>
        {loading ? "Loading..." : "Refresh themes"}
      </button>
      {error && <div style={{ color: "red" }}>{error}</div>}
      <ul>
        {themes.length === 0
          ? <li>No themes found</li>
          : themes.map((t) => (
            <li key={t}>
              {t} <button onClick={() => applyTheme(t)}>Apply</button>
            </li>
          ))}
      </ul>
    </div>
  );
}
