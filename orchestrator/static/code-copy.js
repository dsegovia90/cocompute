// Copy-to-clipboard buttons for [data-codeblock] (progressive enhancement).

(function () {
  "use strict";

  function legacyCopy(text) {
    try {
      var ta = document.createElement("textarea");
      ta.value = text;
      ta.setAttribute("readonly", "");
      ta.style.position = "absolute";
      ta.style.left = "-9999px";
      document.body.appendChild(ta);
      ta.select();
      var ok = document.execCommand("copy");
      document.body.removeChild(ta);
      return ok;
    } catch (e) {
      return false;
    }
  }

  function init() {
    document.querySelectorAll("[data-codeblock] .copy-btn").forEach(function (btn) {
      btn.addEventListener("click", function () {
        var block = btn.closest("[data-codeblock]");
        var pre = block && block.querySelector("pre");
        if (!pre) return;
        var text = pre.textContent || "";

        var done = function (ok) {
          if (!ok) return;
          var copyIcon = btn.querySelector(".copy-icon");
          var checkIcon = btn.querySelector(".check-icon");
          if (copyIcon) copyIcon.classList.add("hidden");
          if (checkIcon) checkIcon.classList.remove("hidden");
          setTimeout(function () {
            if (copyIcon) copyIcon.classList.remove("hidden");
            if (checkIcon) checkIcon.classList.add("hidden");
          }, 1500);
        };

        // navigator.clipboard is secure-context only; fall back for http.
        if (navigator.clipboard && navigator.clipboard.writeText) {
          navigator.clipboard.writeText(text).then(function () { done(true); }, function () {
            done(legacyCopy(text));
          });
        } else {
          done(legacyCopy(text));
        }
      });
    });
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();
