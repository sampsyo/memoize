const source = new EventSource("/_notify");
source.onmessage = (event) => {
  if (event.data === "reload") {
    location.reload();
  }
};
