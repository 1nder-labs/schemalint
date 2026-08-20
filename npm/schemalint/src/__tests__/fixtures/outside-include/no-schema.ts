// Fixture for U6: exists on disk but outside the fixtures tsconfig `include`
// list (`"include": ["*.ts"]` only reaches top-level files). Used to prove
// that discovery distinguishes "no file on disk" from "file on disk but
// outside the TypeScript program".
export const marker = 'outside-include';
