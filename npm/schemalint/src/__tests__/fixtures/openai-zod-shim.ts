export function zodTextFormat(schema: unknown, name: string): unknown {
  return { schema, name };
}

export function zodResponseFormat(schema: unknown, name: string): unknown {
  return { schema, name };
}

export function zodFunction(args: {
  name: string;
  parameters: unknown;
}): unknown {
  return args;
}
