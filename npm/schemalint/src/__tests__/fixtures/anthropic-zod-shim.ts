export function betaZodTool(args: {
  name: string;
  inputSchema: unknown;
}): unknown {
  return args;
}


export function zodOutputFormat(schema: unknown): unknown {
  return { schema };
}
