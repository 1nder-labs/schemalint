declare module 'picomatch' {
  function picomatch(
    pattern: string,
    options?: { dot?: boolean }
  ): (input: string) => boolean;
  namespace picomatch {
    function scan(pattern: string): { base: string; glob: string };
  }
  export = picomatch;
}
