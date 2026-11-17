// The page's host for the web tests: it lets the app reload its own page, so a test can read
// what an earlier load of the same page wrote. Each `open` gets storage of its own.
export async function createHost() {
  return {
    call(name) {
      throw new Error(`no host function ${name}`);
    },
    post(name) {
      if (name === "page.reload") location.reload();
    },
  };
}
