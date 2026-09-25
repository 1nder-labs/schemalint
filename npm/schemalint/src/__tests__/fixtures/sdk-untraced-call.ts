import { generateText } from 'ai';

// Uses the provider SDK with no structured-output schema at all.
export async function run() {
  return generateText({ model: {} as never, prompt: 'x' });
}
