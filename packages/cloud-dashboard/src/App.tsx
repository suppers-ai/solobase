import { useState, useEffect, useCallback } from "preact/hooks";
import {
  Key,
  Settings,
  LogOut,
  CreditCard,
  Server,
  Plus,
  Trash2,
  Rocket,
  Shield,
  Clock,
  XCircle,
  Activity,
  BarChart3,
} from "lucide-preact";
import { api } from "./api";
import { checkAuth, getUser, getRoles, isAdmin, logout } from "./auth";
import { toasts, getToasts, onToastsChange } from "./toast";
import "./style.css";

// ─── Plan limits ────────────────────────────────────────────────────
const PLAN_LIMITS: Record<
  string,
  {
    requests: number;
    r2: number;
    d1: number;
    maxCreated: number;
    maxActive: number;
  }
> = {
  free: { requests: 0, r2: 0, d1: 0, maxCreated: 2, maxActive: 0 },
  starter: {
    requests: 500000,
    r2: 2 * 1024 ** 3,
    d1: 500 * 1024 ** 2,
    maxCreated: 2,
    maxActive: 2,
  },
  pro: {
    requests: 3000000,
    r2: 20 * 1024 ** 3,
    d1: 5 * 1024 ** 3,
    maxCreated: 10,
    maxActive: 10,
  },
  platform: {
    requests: Infinity,
    r2: Infinity,
    d1: Infinity,
    maxCreated: Infinity,
    maxActive: Infinity,
  },
};

// ─── Tiny Components ────────────────────────────────────────────────
function Spinner({ message }: { message?: string }) {
  return <div class="spinner">{message || "Loading..."}</div>;
}

function ToastContainer() {
  const [, setTick] = useState(0);
  useEffect(() => {
    onToastsChange(() => setTick((t) => t + 1));
  }, []);
  return (
    <div class="toast-container">
      {getToasts().map((t) => (
        <div key={t.id} class={`toast toast-${t.type}`}>
          {t.message}
        </div>
      ))}
    </div>
  );
}

function StatusBadge({ status }: { status: string }) {
  const cls =
    status === "active"
      ? "badge-success"
      : status === "pending" || status === "inactive"
        ? "badge-warning"
        : status === "stopped"
          ? "badge-danger"
          : "badge-neutral";
  return <span class={`badge ${cls}`}>{status}</span>;
}

// ─── Auth Guard ─────────────────────────────────────────────────────
function AuthGuard({ children }: { children: any }) {
  const [checked, setChecked] = useState(false);
  useEffect(() => {
    checkAuth().then((ok) => {
      if (!ok)
        window.location.href =
          "/b/auth/login?redirect=" + encodeURIComponent(window.location.href);
      else setChecked(true);
    });
  }, []);
  if (!checked) return <Spinner message="Loading..." />;
  return children;
}

// ─── Header ─────────────────────────────────────────────────────────
function Header() {
  const user = getUser();
  return (
    <header
      style={{
        display: "flex",
        justifyContent: "space-between",
        alignItems: "center",
        padding: "1rem 1.5rem",
        background: "white",
        borderBottom: "1px solid var(--border)",
      }}
    >
      <img
        src="/images/logo_long.png"
        alt="Solobase"
        style={{ height: 32, width: "auto" }}
      />
      <div style={{ display: "flex", alignItems: "center", gap: "1rem" }}>
        <span style={{ fontSize: "0.813rem", color: "var(--text-muted)" }}>
          {user?.email}
        </span>
        <button
          class="btn btn-ghost btn-sm"
          onClick={() => {
            logout();
            window.location.href = "/b/auth/login";
          }}
        >
          <LogOut size={14} /> Logout
        </button>
      </div>
    </header>
  );
}

// ─── Nav ────────────────────────────────────────────────────────────
function Nav({
  active,
  onNavigate,
}: {
  active: string;
  onNavigate: (p: string) => void;
}) {
  const isAdmin = getRoles().includes("admin");
  const tabs = [
    { id: "overview", label: "Overview", Icon: Activity },
    { id: "projects", label: "Projects", Icon: Server },
    { id: "api-keys", label: "API Keys", Icon: Key },
    { id: "settings", label: "Settings", Icon: Settings },
    ...(isAdmin ? [{ id: "admin", label: "Admin", Icon: Shield }] : []),
  ];
  return (
    <nav
      style={{
        padding: "0 1.5rem",
        background: "white",
        borderBottom: "1px solid var(--border)",
      }}
    >
      <div class="tabs">
        {tabs.map((t) => (
          <button
            key={t.id}
            class={`tab ${active === t.id ? "active" : ""}`}
            onClick={() => onNavigate(t.id)}
          >
            <t.Icon size={16} /> {t.label}
          </button>
        ))}
      </div>
    </nav>
  );
}

// ─── Overview ───────────────────────────────────────────────────────
function OverviewTab() {
  const user = getUser();
  const [planName, setPlanName] = useState("...");
  const [projectCount, setProjectCount] = useState("...");
  const [apiKeyCount, setApiKeyCount] = useState("0");

  useEffect(() => {
    api
      .get("/b/products/subscription")
      .then((d: any) => {
        const p = d?.subscription?.plan || "free";
        setPlanName(p === "pro" ? "Pro" : p === "starter" ? "Starter" : "Free");
      })
      .catch(() => setPlanName("Free"));
    api
      .get("/b/projects")
      .then((d: any) => {
        const r = Array.isArray(d?.records)
          ? d.records
          : Array.isArray(d)
            ? d
            : [];
        setProjectCount(String(r.length));
      })
      .catch(() => setProjectCount("0"));
    api
      .get("/b/auth/api/api-keys")
      .then((d: any) => {
        const k = Array.isArray(d?.records)
          ? d.records
          : Array.isArray(d)
            ? d
            : [];
        setApiKeyCount(String(k.length));
      })
      .catch(() => setApiKeyCount("0"));
  }, []);

  const name = user?.name || user?.email?.split("@")[0] || "there";
  return (
    <div>
      <div class="page-header">
        <h1>Welcome back, {name}</h1>
        <p>Here's an overview of your account</p>
      </div>
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fill, minmax(220px, 1fr))",
          gap: "1rem",
          marginBottom: "2rem",
        }}
      >
        <div class="stat-card">
          <div class="stat-label">Plan</div>
          <div class="stat-value">{planName}</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">Projects</div>
          <div class="stat-value">{projectCount}</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">API Keys</div>
          <div class="stat-value">{apiKeyCount}</div>
        </div>
      </div>
      <div style={{ display: "flex", gap: "0.75rem" }}>
        <a href="#projects" class="btn btn-primary">
          <Rocket size={16} /> Create Project
        </a>
        <a href="https://solobase.dev/docs/" class="btn btn-secondary">
          Read Docs
        </a>
      </div>
    </div>
  );
}

// ─── Projects ───────────────────────────────────────────────────────
function ProjectsTab() {
  const [projects, setProjects] = useState<any[]>([]);
  const [plan, setPlan] = useState("free");
  const [loading, setLoading] = useState(true);
  const [showForm, setShowForm] = useState(false);
  const [newName, setNewName] = useState("");
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetch_ = useCallback(async () => {
    try {
      const d: any = await api.get("/b/projects");
      setProjects(
        Array.isArray(d?.records) ? d.records : Array.isArray(d) ? d : [],
      );
      if (d?.plan) setPlan(d.plan);
    } catch {
      setProjects([]);
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    fetch_();
    api
      .get("/b/products/subscription")
      .then((d: any) => {
        const p = d?.subscription?.plan;
        if (p) setPlan(p);
      })
      .catch(() => {});
  }, [fetch_]);

  function validate(v: string): string | null {
    if (!v) return null;
    if (v.length < 3) return "Min 3 characters";
    if (v.length > 63) return "Max 63 characters";
    if (!/^[a-z]/.test(v)) return "Must start with lowercase letter";
    if (!/^[a-z0-9-]+$/.test(v))
      return "Only lowercase letters, numbers, hyphens";
    if (v.endsWith("-") || v.includes("--")) return "Invalid hyphen placement";
    return null;
  }

  async function handleCreate(e: Event) {
    e.preventDefault();
    const err = validate(newName);
    if (err) {
      setError(err);
      return;
    }
    setCreating(true);
    try {
      await api.post("/b/projects", { name: newName.trim() });
      toasts.success("Project created");
      setNewName("");
      setShowForm(false);
      setError(null);
      await fetch_();
    } catch (e: any) {
      setError(e.message);
    }
    setCreating(false);
  }

  async function handleDelete(id: string) {
    try {
      await api.delete(`/b/projects/${id}`);
      toasts.success("Project deleted");
      await fetch_();
    } catch (e: any) {
      toasts.error(e.message);
    }
  }

  async function handleActivate(id: string) {
    try {
      await api.put(`/b/projects/${id}`, { action: "activate" });
      toasts.success("Project activated");
      await fetch_();
    } catch (e: any) {
      toasts.error(e.message);
    }
  }

  async function handleDeactivate(id: string) {
    try {
      await api.put(`/b/projects/${id}`, { action: "deactivate" });
      toasts.success("Project deactivated");
      await fetch_();
    } catch (e: any) {
      toasts.error(e.message);
    }
  }

  if (loading) return <Spinner message="Loading projects..." />;
  const limits = PLAN_LIMITS[plan] || PLAN_LIMITS.free;

  return (
    <div>
      <div
        class="page-header"
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "flex-start",
        }}
      >
        <div>
          <h1>Projects</h1>
          <p>Manage your backend instances</p>
        </div>
        <button class="btn btn-primary" onClick={() => setShowForm(true)}>
          <Plus size={16} /> Create Project
        </button>
      </div>

      {showForm && (
        <div class="card" style={{ marginBottom: "1.5rem" }}>
          <h3
            style={{ fontSize: "1rem", fontWeight: 600, marginBottom: "1rem" }}
          >
            New Project
          </h3>
          <form onSubmit={handleCreate}>
            <label class="label">Subdomain</label>
            <input
              class="input"
              type="text"
              value={newName}
              onInput={(e: any) => {
                setNewName(
                  e.target.value.toLowerCase().replace(/[^a-z0-9-]/g, ""),
                );
                setError(null);
              }}
              placeholder="my-app"
              required
              style={error ? { borderColor: "var(--danger)" } : {}}
            />
            {newName && (
              <div
                style={{
                  fontSize: "0.813rem",
                  color: "var(--text-muted)",
                  marginTop: "0.375rem",
                }}
              >
                Available at{" "}
                <span style={{ color: "var(--accent)", fontWeight: 600 }}>
                  {newName}.solobase.dev
                </span>
              </div>
            )}
            {error && (
              <div
                style={{
                  fontSize: "0.75rem",
                  color: "var(--danger)",
                  marginTop: "0.25rem",
                }}
              >
                {error}
              </div>
            )}
            <div style={{ display: "flex", gap: "0.5rem", marginTop: "1rem" }}>
              <button
                class="btn btn-primary"
                type="submit"
                disabled={creating || !!validate(newName)}
              >
                {creating ? "Creating..." : "Create"}
              </button>
              <button
                class="btn btn-secondary"
                type="button"
                onClick={() => {
                  setShowForm(false);
                  setNewName("");
                  setError(null);
                }}
              >
                Cancel
              </button>
            </div>
          </form>
        </div>
      )}

      {projects.length === 0 && !showForm && (
        <div class="card">
          <div class="empty-state">
            <Server size={40} color="var(--text-light)" />
            <h3>No projects yet</h3>
            <p>Create your first Solobase backend instance.</p>
            <button class="btn btn-primary" onClick={() => setShowForm(true)}>
              <Rocket size={16} /> Create Project
            </button>
          </div>
        </div>
      )}

      <div style={{ display: "grid", gap: "0.5rem" }}>
        {projects
          .filter((d: any) => (d.data?.status || d.status) !== "deleted")
          .map((d: any) => {
            const st = d.data?.status || d.status || "pending";
            const slug = d.data?.slug || d.data?.name || d.name || "";
            const canActivate =
              plan !== "free" && (st === "pending" || st === "inactive");
            return (
              <div
                key={d.id}
                class="row-item"
                style={{ opacity: st === "active" ? 1 : 0.85 }}
              >
                <div
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: "1rem",
                    flex: 1,
                  }}
                >
                  <Server
                    size={18}
                    style={{
                      color:
                        st === "active"
                          ? "var(--success)"
                          : "var(--text-light)",
                      flexShrink: 0,
                    }}
                  />
                  <div>
                    <div style={{ fontWeight: 600, fontSize: "0.875rem" }}>
                      {d.data?.name || d.name || "Unnamed"}
                    </div>
                    <div
                      style={{
                        fontSize: "0.75rem",
                        color: "var(--text-muted)",
                        marginTop: "0.125rem",
                      }}
                    >
                      {st === "active" && (
                        <>
                          <span style={{ color: "var(--accent)" }}>
                            {slug.toLowerCase().replace(/\s+/g, "-")}
                            .solobase.dev
                          </span>{" "}
                          &middot;{" "}
                        </>
                      )}
                      Created{" "}
                      {d.data?.created_at || d.created_at
                        ? new Date(
                            d.data?.created_at || d.created_at,
                          ).toLocaleDateString()
                        : ""}
                    </div>
                  </div>
                </div>
                <div
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: "0.5rem",
                  }}
                >
                  <StatusBadge status={st} />
                  {canActivate && (
                    <button
                      class="btn btn-sm"
                      style={{ background: "var(--success)", color: "white" }}
                      onClick={() => handleActivate(d.id)}
                    >
                      <Rocket size={12} /> Activate
                    </button>
                  )}
                  {st === "active" && (
                    <button
                      class="btn btn-sm"
                      style={{
                        border: "1px solid var(--warning-border)",
                        color: "var(--warning)",
                      }}
                      onClick={() => handleDeactivate(d.id)}
                    >
                      <XCircle size={12} /> Deactivate
                    </button>
                  )}
                  {st !== "deleted" && (
                    <button
                      class="btn-danger btn"
                      onClick={() => handleDelete(d.id)}
                    >
                      <Trash2 size={12} /> Delete
                    </button>
                  )}
                </div>
              </div>
            );
          })}
      </div>
    </div>
  );
}

// ─── API Keys ───────────────────────────────────────────────────────
function ApiKeysTab() {
  const [keys, setKeys] = useState<any[]>([]);
  const [loading, setLoading] = useState(true);
  const [name, setName] = useState("");
  const [createdKey, setCreatedKey] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);

  const fetch_ = useCallback(async () => {
    try {
      const d: any = await api.get("/b/auth/api/api-keys");
      setKeys(
        Array.isArray(d?.records) ? d.records : Array.isArray(d) ? d : [],
      );
    } catch {
      setKeys([]);
    }
    setLoading(false);
  }, []);
  useEffect(() => {
    fetch_();
  }, [fetch_]);

  async function create(e: Event) {
    e.preventDefault();
    if (!name.trim()) return;
    setCreating(true);
    try {
      const r: any = await api.post("/b/auth/api/api-keys", {
        name: name.trim(),
      });
      setCreatedKey(r.key || r.data?.key);
      setName("");
      await fetch_();
    } catch (e: any) {
      toasts.error(e.message);
    }
    setCreating(false);
  }

  async function revoke(id: string) {
    try {
      await api.delete(`/b/auth/api/api-keys/${id}`);
      toasts.success("API key revoked");
      await fetch_();
    } catch (e: any) {
      toasts.error(e.message);
    }
  }

  if (loading) return <Spinner message="Loading API keys..." />;
  return (
    <div>
      <div class="page-header">
        <h1>API Keys</h1>
        <p>Manage API keys for programmatic access</p>
      </div>
      {createdKey && (
        <div
          style={{
            background: "var(--success-bg)",
            border: "1px solid var(--success-border)",
            borderRadius: "var(--radius)",
            padding: "1rem",
            marginBottom: "1rem",
          }}
        >
          <p
            style={{
              fontSize: "0.813rem",
              fontWeight: 600,
              color: "#166534",
              marginBottom: "0.5rem",
            }}
          >
            New API key created! Copy it now — you won't see it again.
          </p>
          <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
            <code
              style={{
                fontSize: "0.813rem",
                background: "white",
                padding: "0.375rem 0.5rem",
                borderRadius: 4,
                border: "1px solid var(--border)",
                wordBreak: "break-all",
                flex: 1,
              }}
            >
              {createdKey}
            </code>
            <button
              class="btn btn-sm btn-secondary"
              onClick={() => {
                navigator.clipboard.writeText(createdKey);
                toasts.success("Copied");
              }}
            >
              Copy
            </button>
            <button
              class="btn btn-ghost btn-sm"
              onClick={() => setCreatedKey(null)}
            >
              Dismiss
            </button>
          </div>
        </div>
      )}
      <form
        onSubmit={create}
        style={{ display: "flex", gap: "0.5rem", marginBottom: "1.5rem" }}
      >
        <input
          class="input"
          style={{ flex: 1 }}
          value={name}
          onInput={(e: any) => setName(e.target.value)}
          placeholder="Key name (e.g. ci-deploy)"
        />
        <button class="btn btn-primary" type="submit" disabled={creating}>
          <Plus size={16} /> Create Key
        </button>
      </form>
      {keys.length === 0 ? (
        <div class="card">
          <div class="empty-state">
            <Key size={40} color="var(--text-light)" />
            <h3>No API keys</h3>
            <p>Create an API key for programmatic access.</p>
          </div>
        </div>
      ) : (
        <div style={{ display: "grid", gap: "0.5rem" }}>
          {keys.map((k: any) => (
            <div key={k.id} class="row-item">
              <div>
                <div style={{ fontWeight: 600, fontSize: "0.875rem" }}>
                  {k.data?.name || k.name}
                </div>
                <div
                  style={{ fontSize: "0.75rem", color: "var(--text-muted)" }}
                >
                  {k.data?.key_prefix || k.key_prefix} &middot; Created{" "}
                  {k.data?.created_at || k.created_at
                    ? new Date(
                        k.data?.created_at || k.created_at,
                      ).toLocaleDateString()
                    : ""}
                </div>
              </div>
              <button class="btn-danger btn" onClick={() => revoke(k.id)}>
                Revoke
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// ─── Settings ───────────────────────────────────────────────────────
function SettingsTab() {
  const user = getUser();
  const [name, setName] = useState(user?.name || "");
  const [saving, setSaving] = useState(false);
  const [planName, setPlanName] = useState("...");

  useEffect(() => {
    api
      .get("/b/auth/api/me")
      .then((d: any) => {
        if (d?.user?.name) setName(d.user.name);
      })
      .catch(() => {});
    api
      .get("/b/products/subscription")
      .then((d: any) => {
        const p = d?.subscription?.plan;
        setPlanName(p === "pro" ? "Pro" : p === "starter" ? "Starter" : "Free");
      })
      .catch(() => setPlanName("Free"));
  }, []);

  async function save(e: Event) {
    e.preventDefault();
    setSaving(true);
    try {
      await api.put("/b/auth/api/me", { name });
      toasts.success("Profile updated");
      await checkAuth();
    } catch (e: any) {
      toasts.error(e.message);
    }
    setSaving(false);
  }

  return (
    <div>
      <div class="page-header">
        <h1>Account Settings</h1>
        <p>Manage your profile and preferences</p>
      </div>
      <div class="card" style={{ maxWidth: 500, marginBottom: "1.5rem" }}>
        <h3
          style={{
            fontSize: "0.875rem",
            fontWeight: 600,
            marginBottom: "1rem",
          }}
        >
          Current Plan
        </h3>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
          }}
        >
          <span style={{ fontSize: "1.25rem", fontWeight: 700 }}>
            {planName}
          </span>
          <a href="/b/products/my-purchases" class="btn btn-primary">
            Manage Plan
          </a>
        </div>
      </div>
      <div class="card" style={{ maxWidth: 500, marginBottom: "1.5rem" }}>
        <h3
          style={{
            fontSize: "0.875rem",
            fontWeight: 600,
            marginBottom: "1rem",
          }}
        >
          Profile
        </h3>
        <form onSubmit={save}>
          <div style={{ marginBottom: "1rem" }}>
            <label class="label">Email</label>
            <input
              class="input"
              value={user?.email}
              disabled
              style={{ background: "var(--bg)", color: "var(--text-muted)" }}
            />
          </div>
          <div style={{ marginBottom: "1.5rem" }}>
            <label class="label">Display Name</label>
            <input
              class="input"
              value={name}
              onInput={(e: any) => setName(e.target.value)}
              placeholder="Your name"
            />
          </div>
          <button class="btn btn-primary" type="submit" disabled={saving}>
            {saving ? "Saving..." : "Save Changes"}
          </button>
        </form>
      </div>
      <div class="card" style={{ maxWidth: 500 }}>
        <h3
          style={{
            fontSize: "0.875rem",
            fontWeight: 600,
            marginBottom: "0.5rem",
          }}
        >
          Password
        </h3>
        <p
          style={{
            fontSize: "0.813rem",
            color: "var(--text-muted)",
            marginBottom: "1rem",
          }}
        >
          Update your password to keep your account secure.
        </p>
        <a
          href="/b/auth/change-password"
          class="btn"
          style={{ background: "var(--text)", color: "white" }}
        >
          <Shield size={14} /> Change Password
        </a>
      </div>
    </div>
  );
}

// ─── Admin ──────────────────────────────────────────────────────────
function AdminTab() {
  const [subTab, setSubTab] = useState("stats");
  return (
    <div>
      <div
        class="page-header"
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "flex-start",
        }}
      >
        <div>
          <h1>Admin</h1>
          <p>Manage projects across all users</p>
        </div>
        <a href="/b/admin/" class="btn btn-secondary">
          <Shield size={14} /> Open Admin Panel
        </a>
      </div>
      <div class="tabs" style={{ marginBottom: "1.5rem" }}>
        <button
          class={`tab ${subTab === "stats" ? "active" : ""}`}
          onClick={() => setSubTab("stats")}
        >
          <BarChart3 size={16} /> Stats
        </button>
        <button
          class={`tab ${subTab === "projects" ? "active" : ""}`}
          onClick={() => setSubTab("projects")}
        >
          <Rocket size={16} /> All Projects
        </button>
      </div>
      {subTab === "stats" && <AdminStats />}
      {subTab === "projects" && <AdminProjects />}
    </div>
  );
}

function AdminStats() {
  const [stats, setStats] = useState<any>(null);
  const [loading, setLoading] = useState(true);
  useEffect(() => {
    api
      .get("/b/projects/api/admin/stats")
      .then(setStats)
      .catch(() => {})
      .finally(() => setLoading(false));
  }, []);
  if (loading) return <Spinner />;
  return (
    <div
      style={{
        display: "grid",
        gridTemplateColumns: "repeat(auto-fill, minmax(200px, 1fr))",
        gap: "1rem",
      }}
    >
      <div class="stat-card">
        <div class="stat-label">Total Projects</div>
        <div class="stat-value">{stats?.total ?? 0}</div>
      </div>
      <div class="stat-card">
        <div class="stat-label">Active</div>
        <div class="stat-value" style={{ color: "var(--success)" }}>
          {stats?.active ?? 0}
        </div>
      </div>
      <div class="stat-card">
        <div class="stat-label">Pending</div>
        <div class="stat-value" style={{ color: "var(--warning)" }}>
          {stats?.pending ?? 0}
        </div>
      </div>
      <div class="stat-card">
        <div class="stat-label">Inactive</div>
        <div class="stat-value" style={{ color: "var(--danger)" }}>
          {stats?.inactive ?? 0}
        </div>
      </div>
    </div>
  );
}

function AdminProjects() {
  const [projects, setProjects] = useState<any[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<any>(null);

  useEffect(() => {
    api
      .get("/b/projects/api/admin?pageSize=100")
      .then((d: any) => {
        const records = Array.isArray(d?.records) ? d.records : [];
        setProjects(records.map((r: any) => ({ id: r.id, ...r.data })));
      })
      .catch(() => {})
      .finally(() => setLoading(false));
  }, []);

  const filtered = search
    ? projects.filter(
        (p) =>
          p.name?.toLowerCase().includes(search.toLowerCase()) ||
          p.subdomain?.toLowerCase().includes(search.toLowerCase()),
      )
    : projects;

  if (loading) return <Spinner />;
  return (
    <div>
      <input
        class="input"
        style={{ marginBottom: "1rem", maxWidth: 360 }}
        value={search}
        onInput={(e: any) => setSearch(e.target.value)}
        placeholder="Search by name or subdomain..."
      />
      <div
        style={{
          background: "white",
          border: "1px solid var(--border)",
          borderRadius: 12,
          overflow: "hidden",
        }}
      >
        <table class="data-table">
          <thead>
            <tr>
              <th>Name</th>
              <th>Subdomain</th>
              <th>Status</th>
              <th>Plan</th>
              <th>Created</th>
            </tr>
          </thead>
          <tbody>
            {filtered.map((p) => (
              <tr key={p.id} onClick={() => setSelected(p)}>
                <td style={{ fontWeight: 500 }}>{p.name || "-"}</td>
                <td>{p.subdomain ? `${p.subdomain}.solobase.dev` : "-"}</td>
                <td>
                  <StatusBadge status={p.status || "unknown"} />
                </td>
                <td>{p.plan || "free"}</td>
                <td>
                  {p.created_at
                    ? new Date(p.created_at).toLocaleDateString()
                    : "-"}
                </td>
              </tr>
            ))}
            {filtered.length === 0 && (
              <tr>
                <td
                  colspan={5}
                  style={{
                    textAlign: "center",
                    color: "var(--text-muted)",
                    padding: "2rem",
                  }}
                >
                  No projects found
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
      {selected && (
        <div
          class="modal-overlay"
          onClick={(e: any) => {
            if (e.target.classList.contains("modal-overlay")) setSelected(null);
          }}
        >
          <div class="modal">
            <div class="modal-header">
              <h3>{selected.name}</h3>
              <button class="btn btn-ghost" onClick={() => setSelected(null)}>
                &times;
              </button>
            </div>
            <div class="modal-body">
              {selected.subdomain && (
                <div
                  style={{
                    background: "var(--bg)",
                    borderRadius: "var(--radius)",
                    padding: "0.75rem 1rem",
                    marginBottom: "1rem",
                  }}
                >
                  <a
                    href={`https://${selected.subdomain}.solobase.dev`}
                    target="_blank"
                    style={{ color: "var(--accent)", fontWeight: 600 }}
                  >
                    {selected.subdomain}.solobase.dev
                  </a>
                </div>
              )}
              <div
                style={{
                  display: "grid",
                  gridTemplateColumns: "1fr 1fr",
                  gap: "1rem",
                  marginBottom: "1rem",
                  fontSize: "0.875rem",
                }}
              >
                <div>
                  <div
                    style={{
                      color: "var(--text-light)",
                      fontSize: "0.75rem",
                      marginBottom: "0.25rem",
                    }}
                  >
                    Status
                  </div>
                  <StatusBadge status={selected.status} />
                </div>
                <div>
                  <div
                    style={{
                      color: "var(--text-light)",
                      fontSize: "0.75rem",
                      marginBottom: "0.25rem",
                    }}
                  >
                    Plan
                  </div>
                  {selected.plan || "free"}
                </div>
                <div>
                  <div
                    style={{
                      color: "var(--text-light)",
                      fontSize: "0.75rem",
                      marginBottom: "0.25rem",
                    }}
                  >
                    Created
                  </div>
                  {selected.created_at
                    ? new Date(selected.created_at).toLocaleDateString()
                    : "-"}
                </div>
                <div>
                  <div
                    style={{
                      color: "var(--text-light)",
                      fontSize: "0.75rem",
                      marginBottom: "0.25rem",
                    }}
                  >
                    Updated
                  </div>
                  {selected.updated_at
                    ? new Date(selected.updated_at).toLocaleDateString()
                    : "-"}
                </div>
              </div>
              <div
                style={{
                  borderTop: "1px solid var(--border)",
                  paddingTop: "0.75rem",
                  fontSize: "0.813rem",
                  color: "var(--text-muted)",
                  display: "grid",
                  gridTemplateColumns: "120px 1fr",
                  gap: "0.375rem",
                }}
              >
                <div>Project ID</div>
                <div style={{ color: "var(--text)", wordBreak: "break-all" }}>
                  {selected.id}
                </div>
                {selected.user_id && (
                  <>
                    <div>User ID</div>
                    <div
                      style={{ color: "var(--text)", wordBreak: "break-all" }}
                    >
                      {selected.user_id}
                    </div>
                  </>
                )}
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

// ─── Main Dashboard ─────────────────────────────────────────────────
function Dashboard() {
  const [page, setPage] = useState(
    () => window.location.hash.slice(1) || "overview",
  );

  useEffect(() => {
    window.location.hash = page;
  }, [page]);
  useEffect(() => {
    const onHash = () => setPage(window.location.hash.slice(1) || "overview");
    window.addEventListener("hashchange", onHash);
    return () => window.removeEventListener("hashchange", onHash);
  }, []);

  return (
    <div style={{ minHeight: "100vh", background: "var(--bg)" }}>
      <Header />
      <Nav active={page} onNavigate={setPage} />
      <main style={{ padding: "1.5rem", maxWidth: 1200, margin: "0 auto" }}>
        {page === "overview" && <OverviewTab />}
        {page === "projects" && <ProjectsTab />}
        {page === "api-keys" && <ApiKeysTab />}
        {page === "settings" && <SettingsTab />}
        {page === "admin" && <AdminTab />}
      </main>
      <ToastContainer />
    </div>
  );
}

// ─── App ────────────────────────────────────────────────────────────
export function App() {
  return (
    <AuthGuard>
      <Dashboard />
    </AuthGuard>
  );
}
