// Fixture for U6: valid TypeScript inside the program (matched by the
// fixtures tsconfig `include: ["*.ts"]`) that holds no Zod schema call.
// Used to prove that discovery distinguishes "checked, no schema found"
// from the other two empty-discovery causes.
export const marker = 'no-schema-content';
