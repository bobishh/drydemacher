import {
  expect,
  test,
  type Locator,
  type Page,
  type TestInfo,
} from "@playwright/test";

type VisualMockOptions = {
  textOnly?: boolean;
  terminalAttention?: boolean | null;
};

async function installVisualMock(page: Page, options: VisualMockOptions = {}) {
  await page.addInitScript((mockOptions) => {
    const hasTerminal =
      mockOptions.terminalAttention !== null &&
      mockOptions.terminalAttention !== undefined;
    const model = mockOptions.textOnly
      ? "meta/llama-3.1-70b-instruct"
      : "microsoft/phi-4-multimodal-instruct";
    const config = {
      engines: [
        {
          id: "visual-engine",
          name: "Visual test engine",
          provider: "openai",
          apiKey: "test-key",
          model,
          lightModel: model,
          baseUrl: "https://integrate.api.nvidia.com/v1",
          enabled: true,
        },
      ],
      selectedEngineId: "visual-engine",
      hasSeenOnboarding: true,
      freecadCmd: "",
      assets: [],
      microwave: null,
      mcp: {
        port: null,
        maxSessions: null,
        mode: "passive",
        primaryAgentId: hasTerminal ? "visual-agent" : null,
        promptTimeoutSecs: 1800,
        autoAgents: hasTerminal
          ? [
              {
                id: "visual-agent",
                label: "Codex",
                cmd: "codex",
                args: [],
                enabled: true,
              },
            ]
          : [],
      },
      connectionType: "api_key",
      defaultEngineKind: "build123d",
      defaultSourceLanguage: "ecky",
      defaultGeometryBackend: "build123d",
      maxGenerationAttempts: 3,
      maxVerifyAttempts: 1,
    };

    window.__TAURI_INTERNALS__ = window.__TAURI_INTERNALS__ || {};
    window.__TAURI_INTERNALS__.invoke = async (cmd, args) => {
      if (cmd === "get_config") return config;
      if (cmd === "save_config") return null;
      if (cmd === "get_runtime_capabilities") {
        return {
          freecad: { available: false, detail: "missing", path: null },
          build123d: {
            available: true,
            detail: "Ready",
            path: "/mock/python3",
          },
          mesh: { available: true, detail: "bundled", path: null },
          recommendedAuthoringContext: {
            engineKind: "build123d",
            sourceLanguage: "build123d",
            geometryBackend: "build123d",
          },
        };
      }
      if (cmd === "get_history") return [];
      if (cmd === "get_last_design") return null;
      if (cmd === "list_models") return [model];
      if (cmd === "list_installed_component_package_headers") return [];
      if (cmd === "get_active_agent_sessions") return [];
      if (cmd === "get_agent_terminal_snapshots") {
        if (!hasTerminal) return [];
        return [
          {
            agentId: "visual-agent",
            agentLabel: "Codex",
            sessionId: "visual-session",
            providerKind: "codex",
            sessionNonce: 1,
            screenText: "Approve workspace access?",
            vtStream: "Approve workspace access?",
            vtDelta: null,
            attentionRequired: Boolean(mockOptions.terminalAttention),
            summary: mockOptions.terminalAttention
              ? "Codex needs terminal input."
              : null,
            active: true,
            updatedAt: 100,
          },
        ];
      }
      if (cmd === "get_thread_agent_state") {
        return {
          threadId: args?.threadId ?? null,
          connectionState: "disconnected",
          sessions: [],
          primaryAgentLabel: hasTerminal ? "Codex" : null,
          statusText: "",
        };
      }
      if (cmd === "check_freecad") return false;
      if (cmd === "get_mess_stl_path") return "/mock/mess.stl";
      return null;
    };
  }, options);
}

async function capture(locator: Locator, testInfo: TestInfo, name: string) {
  const path = testInfo.outputPath(`${name}.png`);
  await expect(locator).toBeVisible();
  const clip = await locator.boundingBox();
  if (!clip) throw new Error(`Cannot capture hidden visual target: ${name}`);
  await locator.page().screenshot({ path, clip, animations: "disabled" });
  await testInfo.attach(name, { path, contentType: "image/png" });
}

test.describe("Workbench dock visual state matrix", () => {
  test("captures every persistent icon at neutral, open, focused, and active states", async ({
    page,
  }, testInfo) => {
    test.setTimeout(120_000);
    await installVisualMock(page);
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.goto("/");
    await expect(page.locator(".boot-overlay")).toHaveCount(0);

    const toolbar = page.getByRole("toolbar", { name: "Workbench tools" });
    const controls = [
      ["projects", "Projects"],
      ["params", "Parameters"],
      ["dialogue", "Dialogue"],
      ["code", "Code inspector"],
      ["docs", "Ecky IR docs"],
      ["library", "Reusable component library"],
      ["draw", "Draw annotations"],
      ["settings", "Settings"],
    ] as const;

    await capture(toolbar, testInfo, "dock-neutral-matrix");
    for (const [id, name] of controls) {
      const control = toolbar.getByRole("button", { name, exact: true });
      await expect(control).toHaveAttribute(
        "data-state",
        "closed",
      );
      await capture(control, testInfo, `dock-neutral-${id}`);
    }

    const launchers = controls.filter(
      ([id]) => id !== "draw",
    );
    for (const [id, name] of launchers) {
      await page.reload();
      await expect(page.locator(".boot-overlay")).toHaveCount(0);
      const launcherToolbar = page.getByRole("toolbar", {
        name: "Workbench tools",
      });
      const control = launcherToolbar.getByRole("button", {
        name,
        exact: true,
      });
      const backgroundLauncher = id === "params" ? "Projects" : "Parameters";
      const backgroundControl = launcherToolbar.getByRole("button", {
        name: backgroundLauncher,
        exact: true,
      });
      await control.click();
      await expect(control).toHaveAttribute("data-state", "focused");
      await capture(control, testInfo, `dock-focused-${id}`);

      await backgroundControl.click();
      await expect(backgroundControl).toHaveAttribute("data-state", "focused");
      await expect(control).toHaveAttribute("data-state", "open");
      await capture(control, testInfo, `dock-open-${id}`);
    }

    await page.reload();
    await expect(page.locator(".boot-overlay")).toHaveCount(0);
    const dialogue = toolbar.getByRole("button", {
      name: "Dialogue",
      exact: true,
    });
    await dialogue.click();
    await expect(dialogue).toHaveAttribute("data-state", "focused");
    const draw = toolbar.getByRole("button", {
      name: "Draw annotations",
      exact: true,
    });
    await draw.click();
    await expect(draw).toHaveAttribute("data-state", "activeMode");
    await capture(draw, testInfo, "dock-active-draw");

    await page.mouse.move(2, 2);
    await capture(
      page.getByRole("application"),
      testInfo,
      "workbench-normal-dialogue-geometry",
    );
    await page.setViewportSize({ width: 640, height: 480 });
    await expect(toolbar).toBeVisible();
    await capture(
      page.getByRole("application"),
      testInfo,
      "workbench-compact-dialogue-geometry",
    );
  });

  test("captures disabled Draw state", async ({
    page,
  }, testInfo) => {
    await installVisualMock(page, { textOnly: true });
    await page.goto("/");
    await expect(page.locator(".boot-overlay")).toHaveCount(0);

    const toolbar = page.getByRole("toolbar", { name: "Workbench tools" });
    const draw = toolbar.getByRole("button", {
      name: "Draw annotations",
      exact: true,
    });
    await expect(draw).toHaveAttribute("data-state", "disabled");
    await capture(toolbar, testInfo, "dock-disabled-matrix");
    await capture(draw, testInfo, "dock-disabled-draw");
  });

  for (const [label, attentionRequired] of [
    ["neutral-open-focused", false],
    ["attention", true],
  ] as const) {
    test(`captures Terminal ${label} state`, async ({ page }, testInfo) => {
      await installVisualMock(page, { terminalAttention: attentionRequired });
      await page.goto("/");
      await expect(page.locator(".boot-overlay")).toHaveCount(0);

      const terminal = page
        .getByRole("toolbar", { name: "Workbench tools" })
        .getByRole("button", { name: "Agent terminal", exact: true });
      await expect(terminal).toHaveAttribute(
        "data-state",
        attentionRequired ? "attention" : "closed",
      );
      if (attentionRequired) {
        await capture(terminal, testInfo, "dock-attention-terminal");
        return;
      }

      await capture(terminal, testInfo, "dock-neutral-terminal");
      await terminal.click();
      await expect(terminal).toHaveAttribute("data-state", "focused");
      await capture(terminal, testInfo, "dock-focused-terminal");
      const params = page
        .getByRole("toolbar", { name: "Workbench tools" })
        .getByRole("button", { name: "Parameters", exact: true });
      await params.click();
      await expect(terminal).toHaveAttribute("data-state", "open");
      await capture(terminal, testInfo, "dock-open-terminal");
    });
  }
});
