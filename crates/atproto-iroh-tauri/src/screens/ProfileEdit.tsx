// Editing your own profile in one Table, after onboarding — the real
// gap the README's "Not yet ported" list used to flag. A NodeProfile is
// per-Table (SPEC.md §3.4: `RecordIdentifier` is `(namespace, author,
// key)`, and `NodeProfile::SELF_KEY` is a fixed per-author slot within
// one namespace's doc), so there's no single "your profile" to edit —
// only "your profile in this Table," reached from that Table's own
// Members list rather than a global settings screen.

import { useEffect, useState } from "react";
import { useParams, Link } from "react-router-dom";
import { Sticker, STICKER_IDS, type StickerId } from "../components/Sticker";
import { api, type NodeCategory, type NodeProfile } from "../api";

const CATEGORY_LABELS: Record<NodeCategory, string> = {
  cooperative: "Cooperative",
  collective: "Collective",
  mutual_aid: "Mutual aid",
  nonprofit: "Nonprofit",
  informal_group: "Informal group",
  individual: "Individual",
  other: "Other",
};

export function ProfileEdit() {
  const { id } = useParams<{ id: string }>();
  const tableId = id ?? "";

  const [loading, setLoading] = useState(true);
  const [name, setName] = useState("");
  const [category, setCategory] = useState<NodeCategory>("individual");
  const [avatar, setAvatar] = useState<StickerId>("accent");
  const [neighborhood, setNeighborhood] = useState("");
  const [description, setDescription] = useState("");
  // Not editable here — see the comment on the `save()` call below for
  // why this form doesn't expose it, but a Save still shouldn't erase
  // whatever value was already on the synced profile (e.g. a founder's
  // `Some(true)` from create_namespace_with_profile).
  const [governanceEligible, setGovernanceEligible] = useState<boolean | null>(null);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      const did = await api.nodeDid();
      const selfAuthorHex = did?.replace(/^did:iroh:/, "") ?? "";
      const profiles = await api.listProfiles(tableId);
      const mine = profiles.find((p) => p.author_hex === selfAuthorHex)?.profile;
      if (cancelled) return;
      if (mine) {
        setName(mine.name);
        setCategory(mine.category);
        setAvatar((mine.avatar as StickerId) ?? "accent");
        setNeighborhood(mine.neighborhood ?? "");
        setDescription(mine.description ?? "");
        setGovernanceEligible(mine.governance_eligible ?? null);
      }
      setLoading(false);
    })();
    return () => {
      cancelled = true;
    };
  }, [tableId]);

  async function save() {
    if (!name.trim() || saving) return;
    setSaving(true);
    setSaved(false);
    try {
      const profile: Omit<NodeProfile, "created_at"> = {
        name: name.trim(),
        category,
        avatar,
        neighborhood: neighborhood.trim() || null,
        description: description.trim() || null,
        // Round-tripped, not editable from this form — see the field's
        // own comment above. Real decision eligibility comes only from
        // a synced Founding claim or a ratified AdmitCoSigner proposal
        // (fold::fold_namespace), which this field plays no part in
        // despite the name; a checkbox here used to imply otherwise.
        governance_eligible: governanceEligible,
      };
      await api.updateProfile(tableId, profile);
      setSaved(true);
    } finally {
      setSaving(false);
    }
  }

  if (loading) {
    return (
      <div style={{ padding: "var(--space-4xl) var(--space-xl)", color: "var(--text-3)" }}>
        Settling in…
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", minHeight: "100%" }}>
      <div style={{ padding: "var(--space-xl) var(--space-xl) var(--space-md)" }}>
        <Link to={`/table/${tableId}`} style={{ color: "var(--text-3)", fontSize: 13, display: "inline-flex", alignItems: "center", gap: 4, marginBottom: 14 }}>
          ← Back
        </Link>
        <h1 style={{ margin: 0, fontFamily: "var(--font-display)", fontSize: 24, fontWeight: 400 }}>
          Your profile here
        </h1>
      </div>

      <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 10, padding: "6px 0 18px" }}>
        <Sticker id={avatar} size={72} />
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "repeat(4, minmax(0, 1fr))", gap: 14, padding: "0 var(--space-xl) 20px" }}>
        {STICKER_IDS.map((sid) => (
          <button
            key={sid}
            onClick={() => setAvatar(sid)}
            aria-label={`Choose ${sid} sticker`}
            aria-pressed={avatar === sid}
            style={{ background: "none", border: "none", padding: 0, display: "flex", justifyContent: "center" }}
          >
            <Sticker id={sid} size={44} ring={avatar === sid} />
          </button>
        ))}
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: 14, padding: "0 var(--space-xl) var(--space-xl)" }}>
        <Field label="Name">
          <input value={name} onChange={(e) => setName(e.target.value)} style={inputStyle} />
        </Field>

        <Field label="Category">
          <select value={category} onChange={(e) => setCategory(e.target.value as NodeCategory)} style={inputStyle}>
            {Object.entries(CATEGORY_LABELS).map(([value, label]) => (
              <option key={value} value={value}>
                {label}
              </option>
            ))}
          </select>
        </Field>

        <Field label="Neighborhood (optional)">
          <input value={neighborhood} onChange={(e) => setNeighborhood(e.target.value)} style={inputStyle} />
        </Field>

        <Field label="About you (optional)">
          <textarea value={description} onChange={(e) => setDescription(e.target.value)} rows={3} style={{ ...inputStyle, resize: "vertical", fontFamily: "inherit" }} />
        </Field>

        <div style={{ display: "flex", alignItems: "center", gap: 10, marginTop: 6 }}>
          <button
            onClick={save}
            disabled={!name.trim() || saving}
            style={{
              background: "var(--accent)",
              color: "var(--ink)",
              border: "none",
              borderRadius: 12,
              padding: "11px 20px",
              fontSize: 14.5,
              fontWeight: 700,
              opacity: !name.trim() || saving ? 0.5 : 1,
            }}
          >
            {saving ? "Saving…" : "Save"}
          </button>
          {saved && <span style={{ fontSize: 12.5, color: "var(--sage)" }}>Saved</span>}
        </div>
      </div>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label style={{ display: "flex", flexDirection: "column", gap: 6 }}>
      <span style={{ fontSize: 12, fontWeight: 600, color: "var(--text-3)", textTransform: "uppercase", letterSpacing: 0.4 }}>
        {label}
      </span>
      {children}
    </label>
  );
}

const inputStyle: React.CSSProperties = {
  background: "var(--surface)",
  border: "1px solid var(--border)",
  borderRadius: 10,
  color: "var(--text)",
  padding: "9px 12px",
  fontSize: 14,
};
