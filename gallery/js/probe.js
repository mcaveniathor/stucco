customElements.define(
  "st-probe",
  class extends HTMLElement {
    connectedCallback() {
      this.dataset.ready = "";
    }
  },
);
