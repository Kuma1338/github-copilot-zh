(() => {
  'use strict';

  const initialDictionary = globalThis.__COPILOT_ZH_DICTIONARY__ || {};
  const existingRuntime = globalThis.__COPILOT_ZH__;

  if (existingRuntime) {
    if (typeof existingRuntime.updateDictionary === 'function') {
      existingRuntime.updateDictionary(initialDictionary);
    }
    existingRuntime.start();
    return;
  }

  const translationState = globalThis.__COPILOT_ZH_TRANSLATION_STATE__ || {};
  translationState.textNodes = translationState.textNodes || new WeakMap();
  translationState.leafElements = translationState.leafElements || new WeakMap();
  translationState.attributes = translationState.attributes || new WeakMap();
  translationState.compositeElements = translationState.compositeElements || new WeakMap();
  globalThis.__COPILOT_ZH_TRANSLATION_STATE__ = translationState;

  let exact = {};
  let normalizedExact = new Map();
  let patterns = [];
  let excludedSelector = '';
  let observer;

  function escapeRegex(value) {
    return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  }

  function compilePattern(pattern) {
    const names = [];
    const parameterRules = pattern.parameterRules || {};
    const parts = pattern.source.split(/(\{[A-Za-z][A-Za-z0-9_]*\})/g);
    const source = parts.map(part => {
      const match = /^\{([A-Za-z][A-Za-z0-9_]*)\}$/.exec(part);
      if (!match) return escapeRegex(part);
      names.push(match[1]);
      return match[1] === 'count' ? '([0-9][0-9,.]*)' : '(.+?)';
    }).join('');
    return { regex: new RegExp(`^${source}$`), names, target: pattern.target, parameterRules };
  }

  function normalizeWhitespace(value) {
    return value.replace(/\s+/g, ' ').trim();
  }

  function configure(dictionary) {
    const nextDictionary = dictionary || {};
    exact = nextDictionary.exact || {};
    normalizedExact = new Map();
    for (const [source, target] of Object.entries(exact)) {
      const normalized = normalizeWhitespace(source);
      if (normalized) normalizedExact.set(normalized, target);
    }
    patterns = (nextDictionary.patterns || []).map(compilePattern);
    excludedSelector = (nextDictionary.excludedSelectors || []).join(',');
  }

  const conversationSelector = [
    '[data-message-author-role]',
    '[data-testid="conversation-turn"]',
    '[data-testid="chat-message"]'
  ].join(',');

  function elementFor(node) {
    return node && (node.nodeType === 1 ? node : node.parentElement);
  }

  function isTextProtected(node) {
    if (!excludedSelector) return false;
    const element = elementFor(node);
    return Boolean(element && element.closest(excludedSelector));
  }

  function isAttributeProtected(element) {
    if (!excludedSelector || !element) return false;
    const protectedRoot = element.closest(excludedSelector);
    if (!protectedRoot) return false;

    // Conversation metadata and message content are never translated.
    if (element.closest(conversationSelector)) return true;

    // Input labels are application chrome even when the input itself is protected.
    return !element.matches('input, textarea, select, [contenteditable="true"], [role="textbox"]');
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
    const normalized = normalizeWhitespace(content);
    if (normalizedExact.has(normalized)) {
      return `${leading}${normalizedExact.get(normalized)}${trailing}`;
    }
    for (const pattern of patterns) {
      const result = pattern.regex.exec(content);
      if (!result) continue;
      const values = {};
      let valid = true;
      pattern.names.forEach((name, index) => {
        const value = result[index + 1];
        if (pattern.parameterRules[name] === 'exact') {
          if (!Object.prototype.hasOwnProperty.call(exact, value)) {
            valid = false;
            return;
          }
          values[name] = exact[value];
          return;
        }
        values[name] = value;
      });
      if (!valid) continue;
      const translated = pattern.target.replace(/\{([A-Za-z][A-Za-z0-9_]*)\}/g, (_, name) => values[name] ?? `{${name}}`);
      return `${leading}${translated}${trailing}`;
    }
    return value;
  }

  function translateTextNode(node) {
    if (isTextProtected(node) || belongsToStableComposite(node)) return 0;
    const current = node.nodeValue || '';
    const record = translationState.textNodes.get(node);
    const source = record && current === record.translated ? record.source : current;
    const translated = translateValue(source);
    if (translated !== current) node.nodeValue = translated;
    if (record || translated !== source) {
      translationState.textNodes.set(node, { source, translated });
    }
    return translated === current ? 0 : 1;
  }

  function translateLeafElement(element) {
    if (!element || element.nodeType !== 1 || element.childElementCount > 0 || isTextProtected(element)) {
      return 0;
    }
    const current = element.textContent || '';
    const record = translationState.leafElements.get(element);
    const source = record && current === record.translated ? record.source : current;
    const translated = translateValue(source);
    if (translated !== current) element.textContent = translated;
    if (record || translated !== source) {
      translationState.leafElements.set(element, { source, translated });
    }
    return translated === current ? 0 : 1;
  }

  function sourceTextForNode(node) {
    if (!node) return '';
    if (node.nodeType === 3) {
      const current = node.nodeValue || '';
      const record = translationState.textNodes.get(node);
      return record && current === record.translated ? record.source : current;
    }
    if (node.nodeType !== 1) return node.textContent || '';

    const current = node.textContent || '';
    const compositeRecord = translationState.compositeElements.get(node);
    if (compositeRecord && current === compositeRecord.translated) {
      return compositeRecord.source;
    }
    const leafRecord = translationState.leafElements.get(node);
    if (leafRecord && current === leafRecord.translated) return leafRecord.source;
    return [...node.childNodes].map(sourceTextForNode).join('');
  }

  function hasProtectedDescendant(element) {
    return Boolean(excludedSelector && element.querySelector(excludedSelector));
  }

  function belongsToStableComposite(node) {
    let parent = node && node.parentElement;
    while (parent) {
      const record = translationState.compositeElements.get(parent);
      if (record && parent.textContent === record.translated) return true;
      parent = parent.parentElement;
    }
    return false;
  }

  function translateCompositeElement(element) {
    if (
      !element ||
      element.nodeType !== 1 ||
      element.childElementCount === 0 ||
      isTextProtected(element) ||
      hasProtectedDescendant(element)
    ) {
      return 0;
    }

    const source = sourceTextForNode(element);
    const translated = translateValue(source);
    if (translated === source) return 0;

    const childNodes = [...element.childNodes];
    const childElements = childNodes.filter(node => node.nodeType === 1);
    const children = [];
    let sourceCursor = 0;
    let targetCursor = 0;
    for (const child of childElements) {
      const childSource = sourceTextForNode(child);
      const childTranslated = translateValue(childSource);
      if (!childSource || !childTranslated) return 0;

      const sourceIndex = source.indexOf(childSource, sourceCursor);
      const targetIndex = translated.indexOf(childTranslated, targetCursor);
      if (sourceIndex < 0 || targetIndex < 0) return 0;

      const marker = `\uE000${children.length}\uE001`;
      children.push({ child, childSource, childTranslated, marker, targetIndex });
      sourceCursor = sourceIndex + childSource.length;
      targetCursor = targetIndex + childTranslated.length;
    }

    let targetTemplate = '';
    let targetPosition = 0;
    for (const child of children) {
      targetTemplate += translated.slice(targetPosition, child.targetIndex) + child.marker;
      targetPosition = child.targetIndex + child.childTranslated.length;
    }
    targetTemplate += translated.slice(targetPosition);

    let count = 0;
    let childIndex = 0;
    let templatePosition = 0;
    const textSources = new Map(
      childNodes
        .filter(node => node.nodeType === 3)
        .map(node => [node, sourceTextForNode(node)])
    );

    for (const node of childNodes) {
      if (node.nodeType === 8) continue;
      if (node.nodeType === 3) {
        const nextMarker = children[childIndex]?.marker;
        const end = nextMarker
          ? targetTemplate.indexOf(nextMarker, templatePosition)
          : targetTemplate.length;
        if (end < 0) return 0;
        const desired = targetTemplate.slice(templatePosition, end);
        if (node.nodeValue !== desired) {
          node.nodeValue = desired;
          count += 1;
        }
        translationState.textNodes.set(node, {
          source: textSources.get(node),
          translated: desired
        });
        templatePosition = end;
        continue;
      }
      if (node.nodeType !== 1) continue;

      const child = children[childIndex++];
      if (!child || !targetTemplate.startsWith(child.marker, templatePosition)) return 0;
      templatePosition += child.marker.length;
      count += translateLeafElement(child);
      count += translateCompositeElement(child);
    }

    if (templatePosition !== targetTemplate.length) return 0;
    translationState.compositeElements.set(element, { source, translated });
    return count;
  }

  function translateCompositeAncestors(element) {
    let count = 0;
    let current = element;
    while (current) {
      count += translateCompositeElement(current);
      current = current.parentElement;
    }
    return count;
  }

  function translateAttributes(element) {
    if (isAttributeProtected(element)) return 0;
    let count = 0;
    for (const name of ['aria-label', 'title', 'placeholder']) {
      if (!element.hasAttribute(name)) continue;
      const current = element.getAttribute(name);
      const records = translationState.attributes.get(element);
      const record = records && records.get(name);
      const source = record && current === record.translated ? record.source : current;
      const translated = translateValue(source);
      if (translated !== current) {
        element.setAttribute(name, translated);
        count += 1;
      }
      if (record || translated !== source) {
        const nextRecords = records || new Map();
        nextRecords.set(name, { source, translated });
        translationState.attributes.set(element, nextRecords);
      }
    }
    return count;
  }

  function translateRoot(root) {
    if (!root) return 0;
    let count = 0;
    if (root.nodeType === 3) return translateTextNode(root);
    if (root.nodeType !== 1 && root.nodeType !== 9 && root.nodeType !== 11) return 0;

    const elements = [];
    if (root.nodeType === 1) elements.push(root);
    if (root.querySelectorAll) elements.push(...root.querySelectorAll('*'));
    for (const element of elements) {
      count += translateAttributes(element);
      count += translateLeafElement(element);
    }
    for (const element of elements.reverse()) count += translateCompositeElement(element);
    const document = root.ownerDocument || root;
    const walker = document.createTreeWalker(root, 4, {
      acceptNode(node) {
        return isTextProtected(node) ? 2 : 1;
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

  function translateChangedNode(node) {
    const parent = node && node.parentElement;
    let count = 0;
    if (parent && parent.childElementCount === 0) {
      count += translateLeafElement(parent);
    }
    if (parent) count += translateCompositeAncestors(parent);
    if (!count) count += translateTextNode(node);
    return count;
  }

  function start() {
    if (observer || !globalThis.document) return;
    const root = document.documentElement;
    if (!root) return;
    translateRoot(root);
    observer = new MutationObserver(mutations => {
      for (const mutation of mutations) {
        if (mutation.type === 'characterData') translateChangedNode(mutation.target);
        if (mutation.type === 'attributes') translateAttributes(mutation.target);
        if (mutation.type === 'childList') {
          if (mutation.target.nodeType === 1) translateRoot(mutation.target);
          for (const node of mutation.addedNodes || []) {
            const parent = node.parentElement;
            translateRoot(node);
            if (parent) translateCompositeAncestors(parent);
          }
        }
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

  function updateDictionary(dictionary) {
    configure(dictionary);
    if (globalThis.document?.documentElement) {
      return translateRoot(document.documentElement);
    }
    return 0;
  }

  configure(initialDictionary);
  globalThis.__COPILOT_ZH__ = { start, updateDictionary, translateRoot, translateValue };
})();
