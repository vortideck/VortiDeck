const disableScroll = () => {
  // disable scroling but dont delete scroll bar (so it doesnt jump)
  const scrollTop = window.pageYOffset || document.documentElement.scrollTop;
  // Apply 'noscroll' to html instead of to body to prevent double scroll bars in IE
  document.documentElement.classList.add("noscroll");
  document.documentElement.style.top = `-${scrollTop}px`;
  document.body.style.overflow = "hidden";
};

const enableScroll = () => {
  // enable scrolling
  const scrollTop = parseInt(
    document.documentElement.style.top.replace(/[^0-9.]/g, "")
  );
  document.documentElement.classList.remove("noscroll");
  document.body.style.overflow = "";
  document.documentElement.scrollTop = scrollTop;
  document.documentElement.style.top = "";
};

export { disableScroll, enableScroll };
