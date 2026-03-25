import { z } from "zod/v4";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { execCli, parseJsonOutput, parseAuthError } from "./cli.js";
import type { Idea } from "./types.js";

const uuidParam = z.string().regex(/^[0-9a-f-]{6,36}$/i, "Must be a valid UUID or UUID prefix (6-36 hex characters)");

function ideaSummaryLine(idea: Idea): string {
  const title = idea.summaryTitle ?? idea.content.slice(0, 60);
  const status = idea.completedAt ? "done" : (idea.refinementStatus ?? "-");
  const priority = idea.priority ?? "-";
  return `[${idea.uuid.slice(0, 8)}] ${title} (priority: ${priority}, status: ${status})`;
}

function ideaToText(idea: Idea): string {
  const lines: string[] = [];
  lines.push(`# ${idea.summaryTitle ?? "Untitled Idea"}`);
  lines.push("");
  lines.push("## Original Idea");
  lines.push(idea.content);

  if (idea.refinedContent) {
    lines.push("");
    lines.push("## Refined Version");
    lines.push(idea.refinedContent);
  }

  if (idea.refinementSummary) {
    lines.push("");
    lines.push("## Summary");
    lines.push(idea.refinementSummary);
  }

  if (idea.actionSteps?.length) {
    lines.push("");
    lines.push("## Action Steps");
    idea.actionSteps.forEach((step, i) => lines.push(`${i + 1}. ${step}`));
  }

  lines.push("");
  lines.push("## Metadata");
  lines.push(`- UUID: ${idea.uuid}`);
  lines.push(`- Priority: ${idea.priority ?? "-"}`);
  lines.push(`- Status: ${idea.completedAt ? "done" : (idea.refinementStatus ?? "-")}`);
  lines.push(`- Created: ${idea.createdAt}`);
  if (idea.dueDate) lines.push(`- Due: ${idea.dueDate}`);
  if (idea.completedAt) lines.push(`- Completed: ${idea.completedAt}`);

  return lines.join("\n");
}

async function getUpdateNotice(): Promise<string | null> {
  try {
    const result = await execCli(["update", "--check", "--json"]);
    if (result.exitCode !== 0) return null;
    const info = JSON.parse(result.stdout) as {
      current: string;
      latest: string;
      update_available: boolean;
    };
    if (info.update_available) {
      return `\n\n---\n⬆️ Procrast CLI update available: v${info.current} → v${info.latest}. Run \`procrast update\` in the terminal to install.`;
    }
  } catch {
    // Don't let update check failures break tool responses
  }
  return null;
}

function appendUpdateNotice(
  content: Array<{ type: "text"; text: string }>,
  notice: string | null,
): Array<{ type: "text"; text: string }> {
  if (!notice || content.length === 0) return content;
  const last = content[content.length - 1];
  return [...content.slice(0, -1), { type: "text" as const, text: last.text + notice }];
}

export function registerTools(server: McpServer): void {
  server.registerTool("list_ideas", {
    title: "List Ideas",
    description:
      "List ideas from Procrast. Returns idea titles, UUIDs, priorities, and statuses.",
    inputSchema: {
      limit: z.number().default(20).describe("Maximum number of ideas to return"),
      hide_done: z.boolean().default(false).describe("Hide completed ideas"),
    },
    annotations: { readOnlyHint: true },
  }, async ({ limit, hide_done }) => {
    const args = ["list", "--json", "--limit", String(limit)];
    if (hide_done) args.push("--hide-done");

    const [result, updateNotice] = await Promise.all([execCli(args), getUpdateNotice()]);
    const authErr = parseAuthError(result);
    if (authErr) return { content: [{ type: "text", text: authErr }], isError: true };

    try {
      const ideas = parseJsonOutput<Idea[]>(result);
      if (ideas.length === 0) {
        return { content: appendUpdateNotice([{ type: "text", text: "No ideas found." }], updateNotice) };
      }
      const text = ideas.map(ideaSummaryLine).join("\n");
      return {
        content: appendUpdateNotice([{ type: "text", text: `${ideas.length} ideas:\n\n${text}` }], updateNotice),
      };
    } catch (e) {
      return {
        content: [{ type: "text", text: `Failed to list ideas: ${e}` }],
        isError: true,
      };
    }
  });

  server.registerTool("show_idea", {
    title: "Show Idea",
    description: "Show full details of a single idea by UUID or UUID prefix.",
    inputSchema: {
      uuid: uuidParam.describe("Full UUID or prefix (first 6+ characters)"),
    },
    annotations: { readOnlyHint: true },
  }, async ({ uuid }) => {
    const [result, updateNotice] = await Promise.all([execCli(["show", uuid, "--json"]), getUpdateNotice()]);
    const authErr = parseAuthError(result);
    if (authErr) return { content: [{ type: "text", text: authErr }], isError: true };

    try {
      const idea = parseJsonOutput<Idea>(result);
      return { content: appendUpdateNotice([{ type: "text", text: ideaToText(idea) }], updateNotice) };
    } catch (e) {
      const msg = String(e);
      if (msg.includes("Not found") || result.stderr.includes("Not found")) {
        return {
          content: [{ type: "text", text: `Idea not found for UUID: ${uuid}` }],
          isError: true,
        };
      }
      return {
        content: [{ type: "text", text: `Failed to show idea: ${e}` }],
        isError: true,
      };
    }
  });

  server.registerTool("search_ideas", {
    title: "Search Ideas",
    description: "Search ideas by keyword. Returns matching ideas.",
    inputSchema: {
      query: z.string().describe("Search query"),
      limit: z.number().default(20).describe("Maximum results"),
    },
    annotations: { readOnlyHint: true },
  }, async ({ query, limit }) => {
    const [result, updateNotice] = await Promise.all([
      execCli(["search", query, "--json", "--limit", String(limit)]),
      getUpdateNotice(),
    ]);
    const authErr = parseAuthError(result);
    if (authErr) return { content: [{ type: "text", text: authErr }], isError: true };

    try {
      const ideas = parseJsonOutput<Idea[]>(result);
      if (ideas.length === 0) {
        return {
          content: appendUpdateNotice([{ type: "text", text: `No ideas found for "${query}".` }], updateNotice),
        };
      }
      const text = ideas.map(ideaSummaryLine).join("\n");
      return {
        content: appendUpdateNotice([{ type: "text", text: `${ideas.length} results for "${query}":\n\n${text}` }], updateNotice),
      };
    } catch (e) {
      return {
        content: [{ type: "text", text: `Search failed: ${e}` }],
        isError: true,
      };
    }
  });

  server.registerTool("export_idea", {
    title: "Export Idea as Markdown",
    description: "Export an idea as formatted markdown text.",
    inputSchema: {
      uuid: uuidParam.describe("Full UUID or prefix"),
    },
    annotations: { readOnlyHint: true },
  }, async ({ uuid }) => {
    const [result, updateNotice] = await Promise.all([execCli(["show", uuid, "--markdown"]), getUpdateNotice()]);
    const authErr = parseAuthError(result);
    if (authErr) return { content: [{ type: "text", text: authErr }], isError: true };

    if (result.exitCode !== 0) {
      return {
        content: [
          { type: "text", text: result.stderr.trim() || "Export failed" },
        ],
        isError: true,
      };
    }

    return { content: appendUpdateNotice([{ type: "text", text: result.stdout }], updateNotice) };
  });

  server.registerTool("check_update", {
    title: "Check for CLI Updates",
    description:
      "Check if a newer version of the Procrast CLI is available. Returns current and latest version info.",
    annotations: { readOnlyHint: true },
  }, async () => {
    try {
      const result = await execCli(["update", "--check", "--json"]);
      if (result.exitCode !== 0) {
        return {
          content: [{ type: "text", text: `Update check failed: ${result.stderr.trim()}` }],
          isError: true,
        };
      }

      const info = JSON.parse(result.stdout) as {
        current: string;
        latest: string;
        update_available: boolean;
      };

      if (info.update_available) {
        return {
          content: [{
            type: "text",
            text: `Procrast CLI update available: v${info.current} → v${info.latest}. Run \`procrast update\` in the terminal to install.`,
          }],
        };
      }

      return {
        content: [{ type: "text", text: `Procrast CLI is up to date (v${info.current}).` }],
      };
    } catch (e) {
      return {
        content: [{ type: "text", text: `Update check failed: ${e}` }],
        isError: true,
      };
    }
  });

  server.registerTool("check_auth", {
    title: "Check Authentication",
    description:
      "Check if the Procrast CLI is authenticated. If not, provides instructions for logging in.",
    annotations: { readOnlyHint: true },
  }, async () => {
    const result = await execCli(["list", "--limit", "1", "--json"]);
    const authErr = parseAuthError(result);
    if (authErr) return { content: [{ type: "text", text: authErr }], isError: true };

    if (result.exitCode === 0) {
      return {
        content: [{ type: "text", text: "Authenticated and connected to Procrast." }],
      };
    }

    return {
      content: [
        {
          type: "text",
          text: "Unable to connect. The Procrast API may be unreachable.",
        },
      ],
      isError: true,
    };
  });
}
