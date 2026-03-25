import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { registerTools } from "./tools.js";
import { registerResources } from "./resources.js";

const server = new McpServer({
  name: "procrast",
  version: "0.1.1",
});

registerTools(server);
registerResources(server);

const transport = new StdioServerTransport();
await server.connect(transport);
