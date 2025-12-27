const source = new EventSource("/_notify");
source.addEventListener("reload", (event) => {
  location.reload();
});
