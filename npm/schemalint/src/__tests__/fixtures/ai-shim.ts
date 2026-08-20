export function generateObject(args: { schema: unknown }): unknown {
  return args;
}

export function streamObject(args: { schema: unknown }): unknown {
  return args;
}

export function tool(args: { inputSchema: unknown }): unknown {
  return args;
}

export function dynamicTool(args: {
  description?: string;
  inputSchema: unknown;
}): unknown {
  return args;
}

export const Output = {
  object(args: { name?: string; description?: string; schema: unknown }): unknown {
    return args;
  },
  array(args: { name?: string; description?: string; element: unknown }): unknown {
    return args;
  },
};
