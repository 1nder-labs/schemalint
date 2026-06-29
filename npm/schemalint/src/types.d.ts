declare module 'picomatch' {
  function picomatch(
    pattern: string,
    options?: { dot?: boolean }
  ): (input: string) => boolean;
  export = picomatch;
}
