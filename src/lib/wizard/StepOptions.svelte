<p class="callout">I file nuovi e modificati vengono copiati in B. Non cancello mai niente in B.</p>

<style>
  .callout {
    display: flex;
    gap: 12px;
    align-items: center;
    padding: 18px 20px;
    border-radius: var(--r);
    background: var(--success-container);
    color: var(--on-success-container);
    font-weight: 600;
    margin-bottom: 18px;
    font-size: 17.5px;
  }
  .callout::before {
    content: "\2713";
    flex: none;
    width: 36px;
    height: 36px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--ok);
    color: var(--card);
    font-weight: 800;
  }
</style>
