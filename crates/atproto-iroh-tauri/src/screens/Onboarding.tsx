// First-run: pick a sticker and a name — DESIGN_BRIEF.md §4/§5/§6:
// "picking one is part of first-run onboarding, not buried in settings"
// and "should feel like the fun part, not a form field." This screen
// only captures the pick; there's no backend call here yet, because a
// NodeProfile is written per-Table (create_namespace_with_profile /
// update_profile both take one), and no Table exists before onboarding
// finishes — see ProfileDraftContext for how the pick reaches the first
// Table a person creates or joins.

import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { Sticker, type StickerId } from "../components/Sticker";
import { StickerPicker } from "../components/StickerPicker";
import { useProfileDraft } from "../hooks/useProfileDraft";

export function Onboarding() {
  const navigate = useNavigate();
  const { draft, setDraft } = useProfileDraft();
  const [name, setName] = useState(draft.name);
  const [avatar, setAvatar] = useState<StickerId>(draft.avatar ?? "accent");

  function finish() {
    setDraft({ name: name.trim() || "Someone new", avatar });
    navigate("/feed");
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", minHeight: "100%" }}>
      <div style={{ padding: "var(--space-4xl) var(--space-xl) var(--space-sm)" }}>
        <div
          style={{
            width: 34,
            height: 4,
            borderRadius: 2,
            background: "var(--surface-2)",
            marginBottom: "var(--space-xl)",
          }}
        />
        <h1
          style={{
            margin: "0 0 10px",
            fontFamily: "var(--font-display)",
            fontSize: 30,
            lineHeight: 1.25,
            fontWeight: 400,
          }}
        >
          What do you
          <br />
          look like around here?
        </h1>
        <p style={{ margin: 0, fontSize: 14.5, color: "var(--text-2)", lineHeight: 1.55, maxWidth: "29ch" }}>
          Pick a face for your table. You can swap it any time — this is just so people know who's talking.
        </p>
      </div>

      <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 10, padding: "22px 0 10px" }}>
        <Sticker id={avatar} size={92} />
        <input
          aria-label="Your name"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Your name"
          style={{
            background: "transparent",
            border: "none",
            borderBottom: "1.5px solid var(--border)",
            color: "var(--text)",
            font: "600 16px var(--font-body)",
            textAlign: "center",
            padding: "4px 8px",
            width: 160,
          }}
        />
      </div>

      <div style={{ flexGrow: 1, overflowY: "auto", padding: "14px var(--space-xl) var(--space-sm)" }}>
        <StickerPicker value={avatar} onChange={setAvatar} size={60} gap={16} />
      </div>

      <div style={{ padding: "16px var(--space-xl) var(--space-xl)" }}>
        <button
          onClick={finish}
          style={{
            width: "100%",
            background: "var(--accent)",
            color: "var(--ink)",
            border: "none",
            borderRadius: 14,
            padding: 15,
            fontSize: 15.5,
            fontWeight: 700,
          }}
        >
          Looks like me
        </button>
        <p style={{ margin: "12px 0 0", textAlign: "center", fontSize: 12.5, color: "var(--text-3)" }}>
          Custom photos are coming later — stickers for now, on purpose.
        </p>
      </div>
    </div>
  );
}
