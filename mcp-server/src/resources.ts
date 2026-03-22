import { McpServer, ResourceTemplate } from "@modelcontextprotocol/sdk/server/mcp.js";
import { execCli, parseJsonOutput, parseAuthError } from "./cli.js";
import type { Idea } from "./types.js";

export function registerResources(server: McpServer): void {
  // Dynamic resource template for individual ideas
  server.registerResource(
    "idea",
    new ResourceTemplate("procrast://ideas/{uuid}", { list: listIdeas }),
    { description: "A Procrast idea by UUID", mimeType: "text/markdown" },
    async (uri, { uuid }) => {
      const result = await execCli(["show", uuid as string, "--markdown"]);
      const authErr = parseAuthError(result);
      if (authErr) throw new Error(authErr);

      if (result.exitCode !== 0) {
        throw new Error(result.stderr.trim() || "Failed to read idea");
      }

      return { contents: [{ uri: uri.href, mimeType: "text/markdown", text: result.stdout }] };
    }
  );
}

async function listIdeas() {
  const result = await execCli(["list", "--json", "--limit", "100"]);

  try {
    const ideas = parseJsonOutput<Idea[]>(result);
    return {
      resources: ideas.map((idea) => ({
        uri: `procrast://ideas/${idea.uuid}`,
        name: idea.summaryTitle ?? "Untitled Idea",
        description: idea.content.slice(0, 100),
        mimeType: "text/markdown" as const,
      })),
    };
  } catch {
    return { resources: [] };
  }
}
