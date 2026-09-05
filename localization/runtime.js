(() => {
  'use strict';

  if (globalThis.__COPILOT_ZH__) {
    globalThis.__COPILOT_ZH__.start();
    return;
  }

  const dictionary = globalThis.__COPILOT_ZH_DICTIONARY__ || {};
  const exact = dictionary.exact || {};
  const excludedSelectors = dictionary.excludedSelectors || [];
  const excludedSelector = excludedSelectors.join(',');
  let observer;

  function escapeRegex(value) {
    return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  }

  function compilePattern(pattern) {
    const names = [];
    const parts = pattern.source.split(/(\{[A-Za-z][A-Za-z0-9_]*\})/g);
    const source = parts.map(part => {
      const match = /^\{([A-Za-z][A-Za-z0-9_]*)\}$/.exec(part);
      if (!match) return escapeRegex(part);
      names.push(match[1]);
      return match[1] === 'count' ? '([0-9][0-9,.]*)' : '(.+?)';
    }).join('');
    return { regex: new RegExp(`^${source}$`), names, target: pattern.target };
  }

  const patterns = (dictionary.patterns || []).map(compilePattern);

  function isProtected(node) {
    if (!excludedSelector) return false;
    const element = node.nodeType === 1 ? node : node.parentElement;
    return Boolean(element && element.closest(excludedSelector));
  }

  function translateValue(value) {
    const match = /^(\s*)([\s\S]*?)(\s*)$/.exec(value);
    const leading = match[1];
    const content = match[2];
    const trailing = match[3];
    if (!content) return value;
    if (Object.prototype.hasOwnProperty.call(exact, content)) {
      return `${leading}${exact[content]}${trailing}`;
    }
    for (const pattern of patterns) {
      const result = pattern.regex.exec(content);
      if (!result) continue;
      const values = Object.fromEntries(pattern.names.map((name, index) => [name, result[index + 1]]));
      const translated = pattern.target.replace(/\{([A-Za-z][A-Za-z0-9_]*)\}/g, (_, name) => values[name] ?? `{${name}}`);
      return `${leading}${translated}${trailing}`;
    }
    return value;
  }

  function translateTextNode(node) {
    if (isProtected(node)) return 0;
    const translated = translateValue(node.nodeValue || '');
    if (translated === node.nodeValue) return 0;
    node.nodeValue = translated;
    return 1;
  }

  function translateAttributes(element) {
    if (isProtected(element)) return 0;
    let count = 0;
    for (const name of ['aria-label', 'title', 'placeholder']) {
      if (!element.hasAttribute(name)) continue;
      const current = element.getAttribute(name);
      const translated = translateValue(current);
      if (translated !== current) {
        element.setAttribute(name, translated);
        count += 1;
      }
    }
    return count;
  }

  function translateRoot(root) {
    if (!root || isProtected(root)) return 0;
    let count = 0;
    if (root.nodeType === 3) return translateTextNode(root);
    if (root.nodeType !== 1 && root.nodeType !== 9 && root.nodeType !== 11) return 0;

    if (root.nodeType === 1) count += translateAttributes(root);
    const document = root.ownerDocument || root;
    const walker = document.createTreeWalker(root, 4, {
      acceptNode(node) {
        return isProtected(node) ? 2 : 1;
      }
    });
    let node;
    while ((node = walker.nextNode())) count += translateTextNode(node);

    const selector = '[aria-label],[title],[placeholder]';
    if (root.querySelectorAll) {
      for (const element of root.querySelectorAll(selector)) {
        count += translateAttributes(element);
      }
    }
    return count;
  }

  function start() {
    if (observer || !globalThis.document) return;
    translateRoot(document.documentElement);
    observer = new MutationObserver(mutations => {
      for (const mutation of mutations) {
        if (mutation.type === 'characterData') translateTextNode(mutation.target);
        if (mutation.type === 'attributes') translateAttributes(mutation.target);
        for (const node of mutation.addedNodes || []) translateRoot(node);
      }
    });
    observer.observe(document.documentElement, {
      subtree: true,
      childList: true,
      characterData: true,
      attributes: true,
      attributeFilter: ['aria-label', 'title', 'placeholder']
    });
  }

  globalThis.__COPILOT_ZH__ = { start, translateRoot, translateValue };
})();
