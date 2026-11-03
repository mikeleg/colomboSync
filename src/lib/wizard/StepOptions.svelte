<p class="callout">I file nuovi e modificati vengono copiati in B. Non cancello mai niente in B.</p>

<style>
  .callout {
    display: flex;
    gap: 12px;
    align-items: center;
    padding: 16px 18px;
    background: var(--success-container);
    color: var(--on-success-container);
    font-weight: 500;
    margin-bottom: 18px;
    font-size: 18px;
  }
  .callout::before {
    content: "\2713";
    flex: none;
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    background: var(--success);
    color: var(--on-success);
    font-weight: 700;
  }
</style>
