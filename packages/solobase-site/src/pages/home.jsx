import { render } from "preact";
import { useState } from "preact/hooks";
import "../css/main.css";
import Header from "../components/Header";
import Hero from "../components/Hero";
import Footer from "../components/Footer";
import DemoModal from "../components/DemoModal";

const platforms = [
  {
    name: "Linux",
    url: "https://solobase.dev/download/linux",
    extract: "tar xz",
    run: "./solobase",
  },
  {
    name: "Linux ARM",
    url: "https://solobase.dev/download/linux-arm",
    extract: "tar xz",
    run: "./solobase",
  },
  {
    name: "macOS",
    url: "https://solobase.dev/download/mac",
    extract: "tar xz",
    run: "./solobase",
  },
  {
    name: "macOS Intel",
    url: "https://solobase.dev/download/mac-intel",
    extract: "tar xz",
    run: "./solobase",
  },
  {
    name: "Windows",
    url: "https://solobase.dev/download/windows",
    extract: null,
    run: "solobase.exe",
  },
];

function CodeBlock() {
  const [active, setActive] = useState(0);
  const p = platforms[active];
  const comment = { color: "#6a9955" };
  const cmd = { color: "#d4d4d4" };

  return (
    <div
      style={{
        borderRadius: "0.5rem",
        overflow: "hidden",
        background: "#1e1e1e",
      }}
    >
      <div
        style={{
          display: "flex",
          gap: 0,
          borderBottom: "1px solid #2d2d30",
          overflowX: "auto",
        }}
      >
        {platforms.map((pl, i) => (
          <button
            key={pl.name}
            onClick={() => setActive(i)}
            style={{
              padding: "0.5rem 1rem",
              background: i === active ? "#1e1e1e" : "#252526",
              color: i === active ? "#d4d4d4" : "#6b7280",
              border: "none",
              borderBottom:
                i === active ? "2px solid #fe6627" : "2px solid transparent",
              cursor: "pointer",
              fontSize: "0.75rem",
              fontFamily: "'Consolas', 'Monaco', 'Courier New', monospace",
              whiteSpace: "nowrap",
              transition: "color 0.15s",
            }}
          >
            {pl.name}
          </button>
        ))}
      </div>
      <pre
        style={{
          margin: 0,
          padding: "1.25rem 1.5rem",
          fontSize: "0.8rem",
          lineHeight: 1.8,
          whiteSpace: "pre-wrap",
          wordBreak: "break-all",
          color: "#d4d4d4",
          fontFamily: "'Consolas', 'Monaco', 'Courier New', monospace",
        }}
      >
        <span style={comment}># Download the latest release</span>
        {"\n"}
        {p.extract ? (
          <>
            <span style={cmd}>{`curl -sSL ${p.url} | ${p.extract}`}</span>
            {"\n"}
          </>
        ) : (
          <>
            <span style={cmd}>{`curl -sSLO ${p.url}`}</span>
            {"\n"}
            <span style={cmd}>{`tar -xf solobase-windows-amd64.zip`}</span>
            {"\n"}
          </>
        )}
        {"\n"}
        <span style={comment}># Run it</span>
        {"\n"}
        <span style={cmd}>{p.run}</span>
        {"\n\n"}
        <span style={comment}>
          {
            "# That's it. Auth, database, storage, products, admin panel — all running on :8090"
          }
        </span>
      </pre>
    </div>
  );
}

function GetStarted() {
  return (
    <section style={{ background: "#ffffff", padding: "0 1.5rem 4rem" }}>
      <div class="max-w-2xl mx-auto">
        {/* Browser option — easiest path, shown first */}
        <div
          class="mb-8"
          style={{
            background: "#fff7f4",
            border: "2px solid #fe6627",
            borderRadius: "0.75rem",
            padding: "1.5rem 2rem",
          }}
        >
          <div class="flex items-start gap-4">
            <div
              style={{
                flexShrink: 0,
                width: "2.5rem",
                height: "2.5rem",
                borderRadius: "0.5rem",
                background: "#fe6627",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                color: "white",
              }}
            >
              <svg
                style={{ width: "1.25rem", height: "1.25rem" }}
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="1.5"
                  d="M3.055 11H5a2 2 0 012 2v1a2 2 0 002 2 2 2 0 012 2v2.945M8 3.935V5.5A2.5 2.5 0 0010.5 8h.5a2 2 0 012 2 2 2 0 004 0 2 2 0 012-2h1.064M15 20.488V18a2 2 0 012-2h3.064M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
            </div>
            <div style={{ flex: 1 }}>
              <div class="flex items-center gap-2 mb-1">
                <h3 class="font-semibold text-gray-900" style={{ fontSize: "1rem" }}>
                  Browser (No Install)
                </h3>
                <span
                  style={{
                    fontSize: "0.65rem",
                    fontWeight: 600,
                    textTransform: "uppercase",
                    letterSpacing: "0.05em",
                    background: "#fe6627",
                    color: "white",
                    padding: "0.125rem 0.4rem",
                    borderRadius: "0.25rem",
                  }}
                >
                  Easiest
                </span>
              </div>
              <p class="text-sm text-gray-600 mb-3">
                No download. No setup. Runs entirely in your browser.
              </p>
              <a
                href="https://demo.solobase.dev"
                target="_blank"
                rel="noopener noreferrer"
                style={{
                  display: "inline-block",
                  padding: "0.5rem 1.25rem",
                  background: "#fe6627",
                  color: "white",
                  borderRadius: "0.5rem",
                  fontWeight: 600,
                  fontSize: "0.875rem",
                  transition: "background 0.2s",
                }}
                onMouseOver={(e) => (e.currentTarget.style.background = "#e55a1f")}
                onMouseOut={(e) => (e.currentTarget.style.background = "#fe6627")}
              >
                Try Now
              </a>
              <p
                class="text-gray-400"
                style={{ fontSize: "0.75rem", marginTop: "0.75rem" }}
              >
                Data stays in your browser — local only, no sync between devices.
                Storage limited by browser quotas.
              </p>
            </div>
          </div>
        </div>
        <CodeBlock />
      </div>
    </section>
  );
}

function PlatformAgnostic() {
  const items = [
    {
      title: "Run anywhere",
      desc: "Single binary runs on Linux, macOS, Windows — or deploy to Cloudflare Workers. Same code, any platform.",
      icon: (
        <svg
          class="w-6 h-6"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="1.5"
            d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9"
          />
        </svg>
      ),
    },
    {
      title: "Swap backends freely",
      desc: "Switch from SQLite to Postgres, local disk to S3 — just change config.",
      icon: (
        <svg
          class="w-6 h-6"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="1.5"
            d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4"
          />
        </svg>
      ),
    },
    {
      title: "Local-first development",
      desc: "Full backend running on localhost in seconds. Test everything locally before deploying — no cloud account required.",
      icon: (
        <svg
          class="w-6 h-6"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="1.5"
            d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"
          />
        </svg>
      ),
    },
  ];

  return (
    <section style={{ background: "#f9fafb", padding: "4rem 1.5rem" }}>
      <div class="max-w-5xl mx-auto">
        <h2 class="text-3xl font-bold text-center text-gray-900 mb-3">
          Platform Agnostic by Design
        </h2>
        <p class="text-center text-gray-600 mb-10 max-w-2xl mx-auto">
          Build once, deploy anywhere. No vendor lock-in, no rewriting code when
          your infrastructure changes.
        </p>
        <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
          {items.map((item) => (
            <div
              key={item.title}
              class="bg-white rounded-xl p-6"
              style={{ border: "1px solid #e5e7eb" }}
            >
              <div class="mb-3" style={{ color: "#fe6627" }}>
                {item.icon}
              </div>
              <h3 class="font-semibold text-gray-900 mb-1">{item.title}</h3>
              <p class="text-sm text-gray-600 leading-relaxed">{item.desc}</p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

function AgentReady() {
  const points = [
    {
      title: "REST API out of the box",
      desc: "Every block exposes a clean REST API. Agents can create users, query data, upload files, and manage products — all with standard HTTP calls.",
      icon: (
        <svg
          class="w-6 h-6"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="1.5"
            d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"
          />
        </svg>
      ),
    },
    {
      title: "Structured data storage",
      desc: "Collections with automatic schemas, relational queries, and full CRUD. Agents get a real database — not a flat file or key-value hack.",
      icon: (
        <svg
          class="w-6 h-6"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="1.5"
            d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4"
          />
        </svg>
      ),
    },
    {
      title: "Auth built in",
      desc: "JWT tokens, API keys, and role-based access control. Agents can authenticate users and enforce permissions without building auth from scratch.",
      icon: (
        <svg
          class="w-6 h-6"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="1.5"
            d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z"
          />
        </svg>
      ),
    },
    {
      title: "File handling",
      desc: "Upload, organize, and serve files through the API. Agents can store outputs, serve assets, or manage user uploads with built-in quota tracking.",
      icon: (
        <svg
          class="w-6 h-6"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="1.5"
            d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
          />
        </svg>
      ),
    },
    {
      title: "Instant spin-up",
      desc: "One binary, zero dependencies. Spin up a full backend in seconds — perfect for agent workflows that need to provision environments on the fly.",
      icon: (
        <svg
          class="w-6 h-6"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="1.5"
            d="M13 10V3L4 14h7v7l9-11h-7z"
          />
        </svg>
      ),
    },
    {
      title: "Extend with WASM blocks",
      desc: "Need custom logic? Ship it as a WASM block. Agents can use domain-specific tools without forking the backend or waiting for features.",
      icon: (
        <svg
          class="w-6 h-6"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="1.5"
            d="M11 4a2 2 0 114 0v1a1 1 0 001 1h3a1 1 0 011 1v3a1 1 0 01-1 1h-1a2 2 0 100 4h1a1 1 0 011 1v3a1 1 0 01-1 1h-3a1 1 0 01-1-1v-1a2 2 0 10-4 0v1a1 1 0 01-1 1H7a1 1 0 01-1-1v-3a1 1 0 00-1-1H4a2 2 0 110-4h1a1 1 0 001-1V7a1 1 0 011-1h3a1 1 0 001-1V4z"
          />
        </svg>
      ),
    },
  ];

  return (
    <section style={{ background: "#ffffff", padding: "4rem 1.5rem" }}>
      <div class="max-w-5xl mx-auto">
        <h2 class="text-3xl font-bold text-center text-gray-900 mb-3">
          Built for AI Agents
        </h2>
        <p class="text-center text-gray-600 mb-10 max-w-2xl mx-auto">
          AI agents need backends too. Solobase gives them a complete,
          API-driven infrastructure they can provision and use autonomously.
        </p>
        <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-6">
          {points.map((item) => (
            <div
              key={item.title}
              class="rounded-xl p-6"
              style={{ border: "1px solid #e5e7eb" }}
            >
              <div class="mb-3" style={{ color: "#fe6627" }}>
                {item.icon}
              </div>
              <h3 class="font-semibold text-gray-900 mb-1">{item.title}</h3>
              <p class="text-sm text-gray-600 leading-relaxed">{item.desc}</p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

function BottomCTA({ onOpenDemo }) {
  return (
    <section style={{ background: "#f9fafb", padding: "4rem 1.5rem" }}>
      <div class="max-w-2xl mx-auto text-center">
        <h2 class="text-2xl font-bold text-gray-900 mb-6">
          Ready to get started?
        </h2>
        <div class="flex justify-center gap-4 flex-wrap">
          <a
            href="https://demo.solobase.dev"
            target="_blank"
            rel="noopener noreferrer"
            style={{
              display: "inline-block",
              padding: "0.75rem 2rem",
              background: "#1f2937",
              color: "white",
              borderRadius: "0.5rem",
              fontWeight: 600,
              transition: "background 0.2s",
            }}
            onMouseOver={(e) => (e.currentTarget.style.background = "#374151")}
            onMouseOut={(e) => (e.currentTarget.style.background = "#1f2937")}
          >
            Demo
          </a>
          <a
            href="/docs/"
            style={{
              display: "inline-block",
              padding: "0.75rem 2rem",
              background: "white",
              color: "#1f2937",
              border: "2px solid #e5e7eb",
              borderRadius: "0.5rem",
              fontWeight: 600,
              transition: "all 0.2s",
            }}
            onMouseOver={(e) => {
              e.currentTarget.style.borderColor = "#1f2937";
            }}
            onMouseOut={(e) => {
              e.currentTarget.style.borderColor = "#e5e7eb";
            }}
          >
            Read the documentation
          </a>
        </div>
      </div>
    </section>
  );
}

function HomePage() {
  const [demoOpen, setDemoOpen] = useState(false);

  return (
    <>
      <Header onOpenDemo={() => setDemoOpen(true)} />
      <main>
        <Hero />
        <GetStarted />
        <PlatformAgnostic />
        <AgentReady />
        <BottomCTA onOpenDemo={() => setDemoOpen(true)} />
      </main>
      <Footer />
      <DemoModal open={demoOpen} onClose={() => setDemoOpen(false)} />
    </>
  );
}

render(<HomePage />, document.getElementById("app"));
