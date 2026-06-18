(function () {
  const FIXED_TIMESTAMP = "2026-03-20 09:00:00";

  function slugify(value) {
    return String(value ?? "")
      .trim()
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "");
  }

  function deepClone(value) {
    return JSON.parse(JSON.stringify(value));
  }

  function createProduct(id, name, description, vision, goals, tags) {
    return {
      id,
      name,
      description,
      vision,
      goals,
      tags,
      status: "active",
      lifecycle: "active",
      health: "healthy",
      owner_label: "Founder",
      investment_status: "invest",
      roadmap: "Near-term roadmap focuses on features and builder-ready stories.",
      evidence: "Seeded evidence for product-first browser tests.",
      created_at: FIXED_TIMESTAMP,
      updated_at: FIXED_TIMESTAMP,
    };
  }

  function createModule(id, productId, nodeKind, name, description, purpose, sortOrder) {
    return {
      id,
      product_id: productId,
      node_kind: nodeKind,
      name,
      description,
      purpose,
      sort_order: sortOrder,
      created_at: FIXED_TIMESTAMP,
      updated_at: FIXED_TIMESTAMP,
    };
  }

  function createCapability(id, moduleId, parentCapabilityId, level, nodeKind, name, description, acceptanceCriteria, sortOrder) {
    return {
      id,
      module_id: moduleId,
      parent_capability_id: parentCapabilityId,
      level,
      node_kind: nodeKind,
      sort_order: sortOrder,
      name,
      description,
      acceptance_criteria: acceptanceCriteria,
      priority: "medium",
      risk: "low",
      status: "in_progress",
      technical_notes: `${name} technical notes`,
      explanation: `${name} explanation`,
      examples: `${name} examples`,
      implementation_notes: `${name} implementation notes`,
      test_guidance: `${name} test guidance`,
      created_at: FIXED_TIMESTAMP,
      updated_at: FIXED_TIMESTAMP,
    };
  }

  function createWorkItem(
    id,
    productId,
    moduleId,
    capabilityId,
    sourceNodeId,
    sourceNodeType,
    title,
    description,
    workItemType,
    priority,
    status,
    sortOrder,
  ) {
    return {
      id,
      product_id: productId,
      module_id: moduleId,
      capability_id: capabilityId,
      source_node_id: sourceNodeId,
      source_node_type: sourceNodeType,
      parent_work_item_id: null,
      title,
      problem_statement: description,
      description,
      acceptance_criteria: `${title} is verifiable in the UI.`,
      constraints: "",
      work_item_type: workItemType,
      priority,
      complexity: "medium",
      status,
      repo_override_id: null,
      active_repo_id: null,
      branch_name: null,
      sort_order: sortOrder,
      created_at: FIXED_TIMESTAMP,
      updated_at: FIXED_TIMESTAMP,
    };
  }

  const ROOT_ALLOWED_CHILD_KINDS = ["capability"];
  const NESTED_ALLOWED_CHILD_KINDS = ["feature"];

  function createState() {
    const calculatorProductId = "example-product-calculator";
    const wifiProductId = "example-product-wifi-platform";
    const coreMathModuleId = "calc-core-math-engine";
    const wifiModuleId = "wifi-connectivity";
    const expressionCapabilityId = "calc-expression-evaluation";
    const featureCapabilityId = "calc-scientific-mode-feature";
    const securePairingCapabilityId = "wifi-secure-pairing";
    const multiTaskStoryId = "work-item-calc-feature-multi-task-story";

    return {
      nextId: 1,
      settings: {
        "catalog.hide_example_products": "false",
        "planner.channel_preference": "hybrid",
        "planner.escalate_to_call_on_ambiguity": "true",
      },
      providers: [
        {
          id: "provider-deepseek-hosted",
          name: "DeepSeek (Hosted)",
          provider_type: "openai_compatible",
          base_url: "https://api.deepseek.test/v1",
          auth_secret_ref: null,
          enabled: true,
          created_at: FIXED_TIMESTAMP,
          updated_at: FIXED_TIMESTAMP,
        },
      ],
      modelDefinitions: [
        {
          id: "model-deepseek-chat",
          provider_id: "provider-deepseek-hosted",
          name: "deepseek-chat",
          context_window: 128000,
          capability_tags: ["planner", "chat"],
          notes: "Deterministic browser test model",
          enabled: true,
          created_at: FIXED_TIMESTAMP,
          updated_at: FIXED_TIMESTAMP,
        },
      ],
      products: [
        createProduct(
          calculatorProductId,
          "Calculator",
          "A staged React calculator used to pressure-test implementation and validation agents.",
          "Ship calculator outcomes one by one and verify the full autonomous delivery loop.",
          [
            "Validate coding agents against a familiar React app",
            "Exercise testing agents on incremental mathematical outcomes",
          ],
          ["example_product", "seeded_catalog", "react", "calculator"],
        ),
        createProduct(
          wifiProductId,
          "WiFi Platform",
          "Shared connectivity platform used by products that need secure network sync.",
          "Provide reliable, secure device connectivity across product lines.",
          ["Centralize secure pairing", "Support background sync dependencies"],
          ["example_product", "seeded_catalog", "platform", "connectivity"],
        ),
      ],
      modules: [
        createModule(
          coreMathModuleId,
          calculatorProductId,
          "area",
          "Core Math Engine",
          "Semantic root section for the calculator's parsing and evaluation logic.",
          "Coordinate the parser, evaluator, and delivery work attached to the engine.",
          0,
        ),
        createModule(
          wifiModuleId,
          wifiProductId,
          "area",
          "Connectivity Services",
          "Top-level capability for pairing and sync platform behavior.",
          "Own secure connectivity primitives for dependent products.",
          0,
        ),
      ],
      capabilities: [
        createCapability(
          expressionCapabilityId,
          coreMathModuleId,
          null,
          0,
          "capability",
          "Expression Evaluation",
          "Parse tokens, resolve precedence, and produce deterministic calculation results.",
          "Expressions evaluate correctly for chained operators and nested grouping.",
          0,
        ),
        createCapability(
          featureCapabilityId,
          coreMathModuleId,
          expressionCapabilityId,
          1,
          "feature",
          "Scientific Mode Slice",
          "Release advanced evaluation paths without losing the base calculator flow.",
          "Scientific functions are safely introduced behind the feature plan.",
          0,
        ),
        createCapability(
          securePairingCapabilityId,
          wifiModuleId,
          null,
          0,
          "capability",
          "Secure Device Pairing",
          "Pair devices with authenticated exchange and revocation-safe trust boundaries.",
          "Device pairing is reliable, secure, and auditable.",
          0,
        ),
      ],
      strategyNodes: [
        {
          id: "strategy-shared-platforms",
          parent_node_id: null,
          node_kind: "strategic_area",
          name: "Shared Platforms",
          description: "Reusable platforms that support multiple products.",
          owner_label: "Head of Platforms",
          sort_order: 0,
          created_at: FIXED_TIMESTAMP,
          updated_at: FIXED_TIMESTAMP,
        },
        {
          id: "strategy-connectivity",
          parent_node_id: "strategy-shared-platforms",
          node_kind: "domain",
          name: "Device Connectivity",
          description: "Connectivity domain for paired devices and sync.",
          owner_label: "Founder",
          sort_order: 0,
          created_at: FIXED_TIMESTAMP,
          updated_at: FIXED_TIMESTAMP,
        },
      ],
      productStrategyLinks: [
        {
          id: "strategy-link-wifi",
          product_id: wifiProductId,
          strategy_node_id: "strategy-connectivity",
          is_primary: true,
          created_at: FIXED_TIMESTAMP,
        },
      ],
      productDependencies: [
        {
          id: "dependency-calculator-wifi",
          product_id: calculatorProductId,
          capability_id: expressionCapabilityId,
          depends_on_product_id: wifiProductId,
          depends_on_capability_id: securePairingCapabilityId,
          dependency_kind: "platform",
          description: "Expression result sync depends on secure platform pairing.",
          status: "active",
          created_at: FIXED_TIMESTAMP,
          updated_at: FIXED_TIMESTAMP,
        },
      ],
      productReferences: [
        {
          id: "reference-calculator-architecture",
          scope_type: "product",
          scope_id: calculatorProductId,
          title: "Calculator Architecture Note",
          reference_kind: "architecture",
          uri: "docs/calculator-architecture.md",
          content: "Seeded product-level context for product-first reference tests.",
          created_at: FIXED_TIMESTAMP,
          updated_at: FIXED_TIMESTAMP,
        },
        {
          id: "reference-scientific-slice",
          scope_type: "feature",
          scope_id: featureCapabilityId,
          title: "Scientific Mode Evidence",
          reference_kind: "customer_evidence",
          uri: "",
          content: "Evidence attached to a feature scope.",
          created_at: FIXED_TIMESTAMP,
          updated_at: FIXED_TIMESTAMP,
        },
      ],
      workItems: [
        createWorkItem(
          "work-item-calc-product-docs",
          calculatorProductId,
          null,
          null,
          null,
          null,
          "Publish keyboard shortcuts guide",
          "Document product-level shortcuts and usage notes for calculator operators.",
          "review",
          "low",
          "ready_for_review",
          0,
        ),
        createWorkItem(
          "work-item-calc-parser-errors",
          calculatorProductId,
          coreMathModuleId,
          null,
          coreMathModuleId,
          "module",
          "Refine parser error surfaces",
          "Improve direct engine-level error messages for malformed expressions.",
          "refactor",
          "medium",
          "in_progress",
          1,
        ),
        createWorkItem(
          "work-item-calc-precedence",
          calculatorProductId,
          coreMathModuleId,
          expressionCapabilityId,
          expressionCapabilityId,
          "capability",
          "Implement expression precedence resolution",
          "Attach direct delivery work to the semantic capability that owns evaluation logic.",
          "feature",
          "high",
          "in_progress",
          2,
        ),
        createWorkItem(
          "work-item-calc-feature-checklist",
          calculatorProductId,
          coreMathModuleId,
          featureCapabilityId,
          featureCapabilityId,
          "capability",
          "Ship scientific mode slice checklist",
          "Track feature-specific delivery work against the feature instead of the product root.",
          "setup",
          "medium",
          "draft",
          3,
        ),
        createWorkItem(
          multiTaskStoryId,
          calculatorProductId,
          coreMathModuleId,
          featureCapabilityId,
          featureCapabilityId,
          "capability",
          "Make story task lists reliable after task creation",
          "Story used to verify that Product Management renders every child task.",
          "feature",
          "high",
          "draft",
          4,
        ),
        {
          ...createWorkItem(
            "work-item-calc-feature-multi-task-one",
            calculatorProductId,
            coreMathModuleId,
            featureCapabilityId,
            featureCapabilityId,
            "capability",
            "Load selected story tasks through get_sub_work_items",
            "Use the dedicated child-work-item API for the selected story's task panel.",
            "task",
            "high",
            "draft",
            0,
          ),
          parent_work_item_id: multiTaskStoryId,
        },
        {
          ...createWorkItem(
            "work-item-calc-feature-multi-task-two",
            calculatorProductId,
            coreMathModuleId,
            featureCapabilityId,
            featureCapabilityId,
            "capability",
            "Add regression coverage for multi-task story display",
            "Assert that a selected story renders all persisted child tasks.",
            "task",
            "medium",
            "draft",
            1,
          ),
          parent_work_item_id: multiTaskStoryId,
        },
      ],
      repositories: [],
      externalCliRuns: [],
      externalCliRunEvents: [],
      plannerSessions: {},
    };
  }

  function getState() {
    if (!window.__ARUVI_E2E_STATE__) {
      window.__ARUVI_E2E_STATE__ = createState();
    }
    return window.__ARUVI_E2E_STATE__;
  }

  function nextId(prefix) {
    const state = getState();
    const id = `${prefix}-${state.nextId}`;
    state.nextId += 1;
    return id;
  }

  function buildProductTree(productId) {
    const state = getState();
    const product = state.products.find((entry) => entry.id === productId);
    if (!product) {
      return null;
    }
    const modules = state.modules
      .filter((entry) => entry.product_id === productId)
      .sort((a, b) => a.sort_order - b.sort_order)
      .map((moduleEntry) => ({
        module: moduleEntry,
        features: buildCapabilityChildren(moduleEntry.id, null),
      }));
    return { product, modules, roots: buildHierarchyRoots(productId) };
  }

  function buildCapabilityChildren(moduleId, parentCapabilityId) {
    const state = getState();
    return state.capabilities
      .filter((entry) => entry.module_id === moduleId && entry.parent_capability_id === parentCapabilityId)
      .sort((a, b) => a.sort_order - b.sort_order)
      .map((capability) => ({
        capability,
        children: buildCapabilityChildren(moduleId, capability.id),
      }));
  }

  function buildHierarchyRoots(productId) {
    const state = getState();
    return state.modules
      .filter((entry) => entry.product_id === productId)
      .sort((a, b) => a.sort_order - b.sort_order)
      .map((moduleEntry) => ({
        id: moduleEntry.id,
        node_type: "module",
        node_kind: moduleEntry.node_kind,
        module_id: moduleEntry.id,
        capability_id: null,
        parent_node_id: null,
        parent_node_type: null,
        depth: 0,
        name: moduleEntry.name,
        description: moduleEntry.description,
        summary: moduleEntry.purpose,
        path: [moduleEntry.name],
        allowed_child_kinds: ROOT_ALLOWED_CHILD_KINDS,
        children: buildHierarchyCapabilityNodes(moduleEntry.id, null, [moduleEntry.name], 1),
      }));
  }

  function buildHierarchyCapabilityNodes(moduleId, parentCapabilityId, parentPath, depth) {
    const state = getState();
    return state.capabilities
      .filter((entry) => entry.module_id === moduleId && entry.parent_capability_id === parentCapabilityId)
      .sort((a, b) => a.sort_order - b.sort_order)
      .map((capability) => ({
        id: capability.id,
        node_type: "capability",
        node_kind: capability.node_kind,
        module_id: moduleId,
        capability_id: capability.id,
        parent_node_id: parentCapabilityId ?? moduleId,
        parent_node_type: parentCapabilityId ? "capability" : "module",
        depth,
        name: capability.name,
        description: capability.description,
        summary: capability.description,
        path: [...parentPath, capability.name],
        allowed_child_kinds: capability.node_kind === "feature" ? [] : NESTED_ALLOWED_CHILD_KINDS,
        children: buildHierarchyCapabilityNodes(moduleId, capability.id, [...parentPath, capability.name], depth + 1),
      }));
  }

  function listWorkItemsFiltered(filters) {
    const state = getState();
    return state.workItems.filter((item) => {
      if (filters?.productId && item.product_id !== filters.productId) {
        return false;
      }
      if (filters?.moduleId && item.module_id !== filters.moduleId) {
        return false;
      }
      if (filters?.capabilityId && item.capability_id !== filters.capabilityId) {
        return false;
      }
      if (filters?.sourceNodeId && item.source_node_id !== filters.sourceNodeId) {
        return false;
      }
      if (filters?.source_node_id && item.source_node_id !== filters.source_node_id) {
        return false;
      }
      if (filters?.sourceNodeType && item.source_node_type !== filters.sourceNodeType) {
        return false;
      }
      if (filters?.source_node_type && item.source_node_type !== filters.source_node_type) {
        return false;
      }
      if (filters?.status && item.status !== filters.status) {
        return false;
      }
      return true;
    });
  }

  function createWorkItemFromArgs(args) {
    const state = getState();
    const id = nextId("work-item");
    const workItem = {
      id,
      product_id: getArg(args, "productId", "product_id"),
      module_id: getArg(args, "moduleId", "module_id") ?? null,
      capability_id: getArg(args, "capabilityId", "capability_id") ?? null,
      source_node_id: getArg(args, "sourceNodeId", "source_node_id") ?? null,
      source_node_type: getArg(args, "sourceNodeType", "source_node_type") ?? null,
      parent_work_item_id: getArg(args, "parentWorkItemId", "parent_work_item_id") ?? null,
      title: getArg(args, "title") ?? "",
      problem_statement: getArg(args, "problemStatement", "problem_statement") ?? "",
      description: getArg(args, "description") ?? "",
      acceptance_criteria: getArg(args, "acceptanceCriteria", "acceptance_criteria") ?? "",
      constraints: getArg(args, "constraints") ?? "",
      work_item_type: getArg(args, "workItemType", "work_item_type") ?? "feature",
      priority: getArg(args, "priority") ?? "medium",
      complexity: getArg(args, "complexity") ?? "medium",
      status: getArg(args, "status") ?? "draft",
      repo_override_id: null,
      active_repo_id: null,
      branch_name: null,
      sort_order: state.workItems.filter((entry) =>
        entry.product_id === getArg(args, "productId", "product_id")
        && entry.parent_work_item_id === (getArg(args, "parentWorkItemId", "parent_work_item_id") ?? null)
      ).length,
      created_at: FIXED_TIMESTAMP,
      updated_at: FIXED_TIMESTAMP,
    };
    state.workItems.push(workItem);
    return workItem;
  }

  function updateWorkItemFromArgs(args) {
    const state = getState();
    const id = getArg(args, "id");
    const workItem = state.workItems.find((entry) => entry.id === id);
    if (!workItem) {
      throw new Error("Unknown work item");
    }
    if (getArg(args, "title") !== undefined) workItem.title = getArg(args, "title");
    if (getArg(args, "description") !== undefined) workItem.description = getArg(args, "description");
    if (getArg(args, "status") !== undefined) workItem.status = getArg(args, "status");
    if (getArg(args, "problemStatement", "problem_statement") !== undefined) workItem.problem_statement = getArg(args, "problemStatement", "problem_statement");
    if (getArg(args, "acceptanceCriteria", "acceptance_criteria") !== undefined) workItem.acceptance_criteria = getArg(args, "acceptanceCriteria", "acceptance_criteria");
    if (getArg(args, "constraints") !== undefined) workItem.constraints = getArg(args, "constraints");
    workItem.updated_at = FIXED_TIMESTAMP;
    return workItem;
  }

  function deleteWorkItemFromArgs(args) {
    const state = getState();
    const id = getArg(args, "id");
    state.workItems = state.workItems.filter((entry) => entry.id !== id && entry.parent_work_item_id !== id);
    return null;
  }

  function summarizeWorkItemsByProduct() {
    const state = getState();
    return state.products.map((product) => {
      const items = state.workItems.filter((item) => item.product_id === product.id);
      const activeCount = items.filter((item) => !["done", "cancelled", "failed"].includes(item.status)).length;
      return {
        product_id: product.id,
        total_count: items.length,
        active_count: activeCount,
      };
    });
  }

  function createPlannerSessionRecord(args) {
    const sessionId = `planner-session-${slugify(String(Math.random()).slice(2))}-${Date.now()}`;
    const session = {
      session_id: sessionId,
      provider_id: args.providerId ?? args.provider_id ?? null,
      model_name: args.modelName ?? args.model_name ?? null,
      has_pending_plan: false,
      has_draft_plan: false,
      selected_draft_node_id: null,
      selected_product_id: null,
      draftNodes: {},
      draftRootIds: [],
      pending_plan: null,
    };
    getState().plannerSessions[sessionId] = session;
    return session;
  }

  function getPlannerSession(sessionId) {
    const state = getState();
    return state.plannerSessions[sessionId] ?? null;
  }

  function plannerSessionInfo(session) {
    return {
      session_id: session.session_id,
      provider_id: session.provider_id,
      model_name: session.model_name,
      has_pending_plan: session.has_pending_plan,
      has_draft_plan: session.has_draft_plan,
      selected_draft_node_id: session.selected_draft_node_id,
    };
  }

  function createDraftNode(session, type, label, parentId, data) {
    const id = `draft-${type}-${slugify(label)}`;
    if (session.draftNodes[id]) {
      return session.draftNodes[id];
    }
    const node = {
      id,
      type,
      label,
      meta: `draft ${type.replace("_", " ")}`,
      parentId,
      children: [],
      data: data || {},
    };
    session.draftNodes[id] = node;
    if (parentId) {
      const parent = session.draftNodes[parentId];
      if (parent && !parent.children.includes(id)) {
        parent.children.push(id);
      }
    } else if (!session.draftRootIds.includes(id)) {
      session.draftRootIds.push(id);
    }
    session.has_draft_plan = true;
    return node;
  }

  function findDraftNodeById(session, nodeId) {
    if (!nodeId) {
      return null;
    }
    return session.draftNodes[nodeId] ?? null;
  }

  function findDraftNodeByLabel(session, label, type) {
    const normalized = String(label ?? "").trim().toLowerCase();
    return Object.values(session.draftNodes).find((node) => {
      if (type && node.type !== type) {
        return false;
      }
      return node.label.trim().toLowerCase() === normalized;
    }) ?? null;
  }

  function buildDraftTreeNodes(session) {
    function visit(nodeId) {
      const node = session.draftNodes[nodeId];
      if (!node) {
        return null;
      }
      return {
        id: node.id,
        label: node.label,
        meta: node.meta,
        children: node.children.map(visit).filter(Boolean),
      };
    }
    return session.draftRootIds.map(visit).filter(Boolean);
  }

  function makeTraceEvents(input, response, extraEvents) {
    const events = [
      {
        step: 1,
        stage: "input",
        title: "Planner turn context",
        detail: `Latest user request:\n${input}`,
      },
      ...(extraEvents || []),
      {
        step: (extraEvents || []).length + 2,
        stage: "plan",
        title: "Planner plan ready",
        detail: JSON.stringify(response.pending_plan ?? response.assistant_message, null, 2),
      },
    ];
    return events.map((event, index) => ({ ...event, step: index + 1 }));
  }

  function createPlannerPlan(message, actions) {
    return {
      assistant_response: message,
      needs_confirmation: false,
      clarification_question: null,
      actions,
    };
  }

  function createPlannerResponse(session, message, actions, executionLines, selectedNodeId, extraEvents) {
    const response = {
      session_id: session.session_id,
      status: "proposal",
      assistant_message: message,
      pending_plan: createPlannerPlan(message, actions),
      tree_nodes: null,
      draft_tree_nodes: buildDraftTreeNodes(session),
      selected_draft_node_id: selectedNodeId ?? session.selected_draft_node_id ?? null,
      execution_lines: executionLines || [],
      execution_errors: [],
      trace_events: [],
    };
    response.trace_events = makeTraceEvents(executionLines?.[0] ?? message, response, extraEvents);
    session.selected_draft_node_id = response.selected_draft_node_id;
    return response;
  }

  function createPlannerStateResponse(session, status, message, executionLines, selectedNodeId, extraEvents) {
    const response = {
      session_id: session.session_id,
      status,
      assistant_message: message,
      pending_plan: session.pending_plan ?? null,
      tree_nodes: null,
      draft_tree_nodes: buildDraftTreeNodes(session),
      selected_draft_node_id: selectedNodeId ?? session.selected_draft_node_id ?? null,
      execution_lines: executionLines || [],
      execution_errors: [],
      trace_events: [],
    };
    response.trace_events = makeTraceEvents(executionLines?.[0] ?? message, response, extraEvents);
    session.selected_draft_node_id = response.selected_draft_node_id;
    return response;
  }

  function ensureSelectedProductDraft(session, args) {
    const state = getState();
    const selectedProductId = getArg(args || {}, "productId", "product_id") ?? session.selected_product_id ?? state.products[0]?.id ?? null;
    session.selected_product_id = selectedProductId;
    const selectedProduct = state.products.find((product) => product.id === selectedProductId) ?? state.products[0];
    if (!selectedProduct) {
      throw new Error("Select a product before planning.");
    }
    const existing = findDraftNodeByLabel(session, selectedProduct.name, "product");
    if (existing) {
      return existing;
    }

    const product = createDraftNode(session, "product", selectedProduct.name, null, {
      description: selectedProduct.description,
      vision: selectedProduct.vision,
      goals: selectedProduct.goals,
      tags: selectedProduct.tags,
    });

    const inputExperience = createDraftNode(session, "module", "Input Experience", product.id, {
      description: "Handles expression entry, keyboard shortcuts, validation, and accessible input workflows.",
    });
    const calculationHistory = createDraftNode(session, "module", "Calculation History", product.id, {
      description: "Tracks previous calculations, labels, replay, and result comparison.",
    });
    const resultsReporting = createDraftNode(session, "module", "Results Reporting", product.id, {
      description: "Exports calculation summaries, sessions, and reviewable result reports.",
    });
    const expressionEngine = createDraftNode(session, "module", "Expression Engine", product.id, {
      description: "Coordinates parsing, deterministic evaluation, and scientific operations.",
    });

    const validation = createDraftNode(session, "capability", "Expression Input Validation", inputExperience.id, {
      description: "Validate expressions inline and explain recoverable input errors.",
    });
    createDraftNode(session, "work_item", "Build Input Validation UI", validation.id, {
      description: "Create validation affordances for malformed expressions and keyboard input.",
    });

    const historyReview = createDraftNode(session, "capability", "History Review", calculationHistory.id, {
      description: "Review, search, and replay prior calculations.",
    });
    createDraftNode(session, "work_item", "Implement Calculation History", historyReview.id, {
      description: "Persist recent calculations and provide a searchable history panel.",
    });

    const reportExport = createDraftNode(session, "capability", "Result Export", resultsReporting.id, {
      description: "Export result sessions for review and sharing.",
    });
    createDraftNode(session, "work_item", "Build Result Export Review", reportExport.id, {
      description: "Generate a reviewable export for calculation sessions.",
    });

    const deterministicEvaluation = createDraftNode(session, "capability", "Deterministic Evaluation", expressionEngine.id, {
      description: "Evaluate scientific expressions consistently across sessions.",
    });
    createDraftNode(session, "work_item", "Verify Scientific Evaluation", deterministicEvaluation.id, {
      description: "Add deterministic checks for scientific operator precedence and rounding.",
    });

    session.selected_draft_node_id = product.id;
    return product;
  }

  function addNotificationsToProduct(session, selectedNode) {
    const productNode = selectedNode?.type === "product"
      ? selectedNode
      : selectedNode?.type === "module"
        ? findDraftNodeById(session, selectedNode.parentId)
        : selectedNode?.type === "capability"
          ? findDraftNodeById(session, findDraftNodeById(session, selectedNode.parentId)?.parentId)
          : findDraftNodeById(session, findDraftNodeById(session, findDraftNodeById(session, selectedNode?.parentId)?.parentId)?.parentId);

    const resolvedProduct = productNode ?? ensureSelectedProductDraft(session);
    const notificationsModule = createDraftNode(session, "module", "Notifications & Messaging", resolvedProduct.id, {
      description: "Coordinates outbound email, SMS, and WhatsApp notifications across reservations and guest service workflows.",
    });
    const preferencesCapability = createDraftNode(session, "capability", "Guest Notification Preferences", notificationsModule.id, {
      description: "Manage channel preferences, consent, and notification eligibility per guest.",
    });
    createDraftNode(session, "work_item", "Build Notification Preferences UI", preferencesCapability.id, {
      description: "Add settings UI for guest-facing or staff-managed notification preferences and opt-in capture.",
    });
    session.selected_draft_node_id = notificationsModule.id;
    return {
      selectedNodeId: notificationsModule.id,
      actions: [
        {
          type: "create_module",
          target: { productName: resolvedProduct.label },
          name: notificationsModule.label,
          description: notificationsModule.data.description,
        },
        {
          type: "create_capability",
          target: { productName: resolvedProduct.label, moduleName: notificationsModule.label },
          name: preferencesCapability.label,
          description: preferencesCapability.data.description,
        },
      ],
      message: "I expanded the design with a Notifications & Messaging module, including guest preference management so you can handle email and WhatsApp communication cleanly.",
    };
  }

  function enhanceSelectedModule(session, selectedNode) {
    const moduleNode = selectedNode?.type === "module" ? selectedNode : null;
    if (!moduleNode) {
      return null;
    }
    const capabilityA = createDraftNode(session, "capability", "Outbound Delivery Tracking", moduleNode.id, {
      description: "Track send attempts, delivery outcomes, and failed notification retries across channels.",
    });
    const capabilityB = createDraftNode(session, "capability", "Template & Trigger Rules", moduleNode.id, {
      description: "Define message templates and the workflow events that trigger them.",
    });
    session.selected_draft_node_id = capabilityA.id;
    return {
      selectedNodeId: capabilityA.id,
      actions: [
        {
          type: "create_capability",
          target: { moduleName: moduleNode.label },
          name: capabilityA.label,
          description: capabilityA.data.description,
        },
        {
          type: "create_capability",
          target: { moduleName: moduleNode.label },
          name: capabilityB.label,
          description: capabilityB.data.description,
        },
      ],
      message: `I enhanced ${moduleNode.label} with delivery tracking and trigger/template capabilities so the module is operationally useful, not just a placeholder.`,
    };
  }

  function enhanceSelectedCapability(session, selectedNode) {
    const capabilityNode = selectedNode?.type === "capability" ? selectedNode : null;
    if (!capabilityNode) {
      return null;
    }
    const workItem = createDraftNode(session, "work_item", "Implement Delivery Audit Timeline", capabilityNode.id, {
      description: "Build a timeline view for sends, retries, delivery receipts, and manual resend actions.",
    });
    session.selected_draft_node_id = workItem.id;
    return {
      selectedNodeId: workItem.id,
      actions: [
        {
          type: "create_work_item",
          target: { capabilityName: capabilityNode.label },
          title: workItem.label,
          description: workItem.data.description,
        },
      ],
      message: `I added a concrete work item under ${capabilityNode.label} so the capability can move into implementation planning.`,
    };
  }

  function reviseSelectedWorkItem(session, selectedNode) {
    const workItemNode = selectedNode?.type === "work_item" ? selectedNode : null;
    if (!workItemNode) {
      return null;
    }
    workItemNode.label = "Implement Delivery Audit Timeline and Consent Handling";
    workItemNode.data.description = "Expand the delivery audit timeline to include consent state transitions, delivery receipts, retries, and manual resend actions.";
    session.selected_draft_node_id = workItemNode.id;
    return {
      selectedNodeId: workItemNode.id,
      actions: [
        {
          type: "update_work_item",
          target: { workItemTitle: "Implement Delivery Audit Timeline" },
          fields: {
            title: workItemNode.label,
            description: workItemNode.data.description,
          },
        },
      ],
      message: "I revised the selected work item to include consent handling, which makes the messaging work more production-ready.",
    };
  }

  function submitPlannerTurn(args) {
    const sessionId = args.sessionId ?? args.session_id;
    const userInput = String(args.userInput ?? args.user_input ?? "").trim();
    const session = getPlannerSession(sessionId);
    if (!session) {
      throw new Error(`Unknown planner session: ${sessionId}`);
    }

    if (args.selectedDraftNodeId !== undefined || args.selected_draft_node_id !== undefined) {
      session.selected_draft_node_id = args.selectedDraftNodeId ?? args.selected_draft_node_id ?? null;
    }
    if (args.productId !== undefined || args.product_id !== undefined) {
      session.selected_product_id = args.productId ?? args.product_id ?? null;
    }

    const selectedNode = findDraftNodeById(session, session.selected_draft_node_id);
    const normalizedInput = userInput.toLowerCase();

    if (!session.has_draft_plan && (normalizedInput.includes("design") || normalizedInput.includes("module") || normalizedInput.includes("product"))) {
      const product = ensureSelectedProductDraft(session, args);
      const actions = product.children.map((moduleId) => {
          const moduleNode = session.draftNodes[moduleId];
          return {
            type: "create_module",
            target: { productName: product.label },
            name: moduleNode.label,
            description: moduleNode.data.description,
          };
        });
      return createPlannerResponse(
        session,
        `I staged a design for ${product.label} with core modules, foundational capabilities, and starter work items. Use the design tree to refine any branch before applying it.`,
        actions,
        ["Updated the design plan."],
        product.id,
        [
          { stage: "context", title: "Selected product", detail: product.label },
        ],
      );
    }

    if ((normalizedInput.includes("enhance") || normalizedInput.includes("expand")) && selectedNode?.type === "module") {
      const result = enhanceSelectedModule(session, selectedNode);
      return createPlannerResponse(session, result.message, result.actions, ["Updated the design plan."], result.selectedNodeId);
    }

    if ((normalizedInput.includes("add work items") || normalizedInput.includes("implement") || normalizedInput.includes("break this down")) && selectedNode?.type === "capability") {
      const result = enhanceSelectedCapability(session, selectedNode);
      return createPlannerResponse(session, result.message, result.actions, ["Updated the design plan."], result.selectedNodeId);
    }

    if ((normalizedInput.includes("revise") || normalizedInput.includes("consent")) && selectedNode?.type === "work_item") {
      const result = reviseSelectedWorkItem(session, selectedNode);
      return createPlannerResponse(session, result.message, result.actions, ["Updated the design plan."], result.selectedNodeId);
    }

    if (normalizedInput.includes("email") || normalizedInput.includes("whatsapp") || normalizedInput.includes("notification")) {
      const result = addNotificationsToProduct(session, selectedNode);
      return createPlannerResponse(session, result.message, result.actions, ["Updated the design plan."], result.selectedNodeId);
    }

    return {
      session_id: session.session_id,
      status: "clarification",
      assistant_message: "I need a bit more detail. Select a design node or tell me whether you want to expand the product, a module, a capability, or a work item.",
      pending_plan: {
        assistant_response: "I need a bit more detail.",
        needs_confirmation: false,
        clarification_question: "Tell me which part of the design you want to refine next.",
        actions: [],
      },
      tree_nodes: null,
      draft_tree_nodes: buildDraftTreeNodes(session),
      selected_draft_node_id: session.selected_draft_node_id,
      execution_lines: [],
      execution_errors: [],
      trace_events: makeTraceEvents(userInput, { pending_plan: null, assistant_message: "Clarification needed." }, []),
    };
  }

  function persistDraftNodeTree(session) {
    const state = getState();

    function upsertProduct(node) {
      let product = state.products.find((entry) => entry.name === node.label);
      if (!product) {
        product = createProduct(
          nextId("product"),
          node.label,
          node.data.description || "",
          node.data.vision || "",
          node.data.goals || [],
          node.data.tags || [],
        );
        state.products.unshift(product);
      } else {
        product.description = node.data.description || product.description;
        product.vision = node.data.vision || product.vision;
        product.goals = node.data.goals || product.goals;
        product.tags = node.data.tags || product.tags;
        product.updated_at = FIXED_TIMESTAMP;
      }
      return product;
    }

    function upsertModule(node, product) {
      let module = state.modules.find((entry) => entry.product_id === product.id && entry.name === node.label);
      if (!module) {
        module = {
          id: nextId("module"),
          product_id: product.id,
          name: node.label,
          description: node.data.description || "",
          purpose: node.data.description || "",
          sort_order: state.modules.filter((entry) => entry.product_id === product.id).length,
          created_at: FIXED_TIMESTAMP,
          updated_at: FIXED_TIMESTAMP,
        };
        state.modules.push(module);
      } else {
        module.description = node.data.description || module.description;
        module.purpose = node.data.description || module.purpose;
        module.updated_at = FIXED_TIMESTAMP;
      }
      return module;
    }

    function upsertCapability(node, module, level, parentCapabilityId) {
      let capability = state.capabilities.find((entry) => entry.module_id === module.id && entry.name === node.label && entry.parent_capability_id === parentCapabilityId);
      if (!capability) {
        capability = {
          id: nextId("capability"),
          module_id: module.id,
          parent_capability_id: parentCapabilityId,
          level,
          sort_order: state.capabilities.filter((entry) => entry.module_id === module.id && entry.parent_capability_id === parentCapabilityId).length,
          name: node.label,
          description: node.data.description || "",
          acceptance_criteria: node.data.acceptanceCriteria || "",
          priority: "high",
          risk: "medium",
          status: "draft",
          technical_notes: node.data.technicalNotes || "",
          created_at: FIXED_TIMESTAMP,
          updated_at: FIXED_TIMESTAMP,
        };
        state.capabilities.push(capability);
      } else {
        capability.description = node.data.description || capability.description;
        capability.updated_at = FIXED_TIMESTAMP;
      }
      return capability;
    }

    function upsertWorkItem(node, product, module, capability, parentWorkItemId) {
      let workItem = state.workItems.find((entry) => entry.capability_id === capability.id && entry.title === node.label && entry.parent_work_item_id === parentWorkItemId);
      if (!workItem) {
        workItem = {
          id: nextId("work-item"),
          product_id: product.id,
          module_id: module.id,
          capability_id: capability.id,
          parent_work_item_id: parentWorkItemId,
          title: node.label,
          problem_statement: node.data.description || "",
          description: node.data.description || "",
          acceptance_criteria: node.data.acceptanceCriteria || "",
          constraints: "",
          work_item_type: "feature",
          priority: "high",
          complexity: "medium",
          status: "draft",
          repo_override_id: null,
          active_repo_id: null,
          branch_name: null,
          sort_order: state.workItems.filter((entry) => entry.capability_id === capability.id && entry.parent_work_item_id === parentWorkItemId).length,
          created_at: FIXED_TIMESTAMP,
          updated_at: FIXED_TIMESTAMP,
        };
        state.workItems.push(workItem);
      } else {
        workItem.title = node.label;
        workItem.description = node.data.description || workItem.description;
        workItem.problem_statement = node.data.description || workItem.problem_statement;
        workItem.updated_at = FIXED_TIMESTAMP;
      }
      return workItem;
    }

    function walk(nodeId, context) {
      const node = session.draftNodes[nodeId];
      if (!node) {
        return;
      }

      if (node.type === "product") {
        const product = upsertProduct(node);
        node.children.forEach((childId) => walk(childId, { product }));
        return;
      }

      if (node.type === "module" && context.product) {
        const module = upsertModule(node, context.product);
        node.children.forEach((childId) => walk(childId, { product: context.product, module }));
        return;
      }

      if (node.type === "capability" && context.product && context.module) {
        const capability = upsertCapability(node, context.module, context.capability ? context.capability.level + 1 : 0, context.capability?.id ?? null);
        node.children.forEach((childId) => walk(childId, { product: context.product, module: context.module, capability }));
        return;
      }

      if (node.type === "work_item" && context.product && context.module && context.capability) {
        const workItem = upsertWorkItem(node, context.product, context.module, context.capability, context.workItem?.id ?? null);
        node.children.forEach((childId) => walk(childId, { product: context.product, module: context.module, capability: context.capability, workItem }));
      }
    }

    session.draftRootIds.forEach((rootId) => walk(rootId, {}));
  }

  function confirmPlannerPlan(args) {
    const sessionId = args.sessionId ?? args.session_id;
    const session = getPlannerSession(sessionId);
    if (!session) {
      throw new Error(`Unknown planner session: ${sessionId}`);
    }

    persistDraftNodeTree(session);
    session.draftNodes = {};
    session.draftRootIds = [];
    session.has_draft_plan = false;
    session.has_pending_plan = false;
    session.pending_plan = null;
    session.selected_draft_node_id = null;

    return {
      session_id: session.session_id,
      status: "execution",
      assistant_message: "Applied design to catalog.",
      pending_plan: null,
      tree_nodes: null,
      draft_tree_nodes: [],
      selected_draft_node_id: null,
      execution_lines: ["Applied design to the catalog."],
      execution_errors: [],
      trace_events: [
        {
          step: 1,
          stage: "execution",
          title: "Applied staged design",
          detail: "The staged product tree was persisted into products, modules, capabilities, and work items.",
        },
      ],
    };
  }

  function submitPlannerVoiceTurn(args) {
    const sessionId = getArg(args, "sessionId", "session_id");
    const transcript = String(getArg(args, "transcript", "userInput", "user_input") ?? "").trim();
    const session = getPlannerSession(sessionId);
    if (!session) {
      throw new Error(`Unknown planner session: ${sessionId}`);
    }
    if (!transcript) {
      throw new Error("Voice transcript cannot be empty");
    }
    if (args.selectedDraftNodeId !== undefined || args.selected_draft_node_id !== undefined) {
      session.selected_draft_node_id = args.selectedDraftNodeId ?? args.selected_draft_node_id ?? null;
    }
    const normalized = transcript.toLowerCase();

    if ([
      "yes",
      "confirm",
      "go ahead",
      "apply",
      "apply design",
      "apply the design",
      "approve",
      "approve design",
      "approve the design",
      "commit",
      "commit draft",
      "commit the draft",
      "confirm draft",
      "confirm proposal",
      "commit plan",
    ].includes(normalized)) {
      return confirmPlannerPlan(args);
    }

    if ([
      "clear draft",
      "clear the draft",
      "clear proposal",
      "dismiss proposal",
      "dismiss draft",
      "cancel draft",
    ].includes(normalized)) {
      const hadDraft = session.has_draft_plan || session.has_pending_plan;
      session.has_pending_plan = false;
      session.pending_plan = null;
      session.draftNodes = {};
      session.draftRootIds = [];
      session.has_draft_plan = false;
      session.selected_draft_node_id = null;
      return createPlannerStateResponse(
        session,
        "execution",
        hadDraft ? "Cleared the current staged design." : "There is no active design to clear.",
        [hadDraft ? "Cleared the current staged design." : "There is no active design to clear."],
        null,
      );
    }

    const selectMatch = normalized.match(/^(select|choose|highlight|open|expand)\s+(.+)$/);
    if (selectMatch && session.has_draft_plan) {
      const rawTarget = String(transcript.replace(/^(select|choose|highlight|open|expand)\s+/i, "")).trim();
      const normalizedTarget = rawTarget
        .replace(/^work item\s+/i, "")
        .replace(/^work-item\s+/i, "")
        .replace(/^capability\s+/i, "")
        .replace(/^module\s+/i, "")
        .replace(/^product\s+/i, "")
        .replace(/^node\s+/i, "")
        .trim();
      const explicitType = /^work item\s+/i.test(rawTarget)
        ? "work_item"
        : /^work-item\s+/i.test(rawTarget)
          ? "work_item"
          : /^capability\s+/i.test(rawTarget)
            ? "capability"
            : /^module\s+/i.test(rawTarget)
              ? "module"
              : /^product\s+/i.test(rawTarget)
                ? "product"
                : null;
      const node = explicitType
        ? findDraftNodeByLabel(session, normalizedTarget, explicitType)
        : findDraftNodeByLabel(session, normalizedTarget, null);
      if (!node) {
        return createPlannerStateResponse(
          session,
          "session_update",
          `I could not find a design node matching "${rawTarget}".`,
          [],
          session.selected_draft_node_id,
        );
      }
      session.selected_draft_node_id = node.id;
      return createPlannerStateResponse(
        session,
        "session_update",
        `Selected ${node.type.replace("_", " ")} "${node.label}".`,
        [`Selected ${node.type.replace("_", " ")} "${node.label}".`],
        node.id,
      );
    }

    return submitPlannerTurn({
      ...args,
      user_input: transcript,
      userInput: transcript,
    });
  }

  function registerRepository(args) {
    const state = getState();
    const repository = {
      id: nextId("repository"),
      name: String(getArg(args, "name") ?? "repository"),
      local_path: String(getArg(args, "localPath", "local_path") ?? ""),
      remote_url: String(getArg(args, "remoteUrl", "remote_url") ?? ""),
      default_branch: String(getArg(args, "defaultBranch", "default_branch") ?? "main"),
      auth_profile: null,
      created_at: FIXED_TIMESTAMP,
      updated_at: FIXED_TIMESTAMP,
    };
    state.repositories.push(repository);
    return repository;
  }

  function analyzeRepositoryForPlanner(args) {
    const sessionId = getArg(args, "sessionId", "session_id");
    const repositoryId = getArg(args, "repositoryId", "repository_id");
    const session = getPlannerSession(sessionId);
    const repository = getState().repositories.find((entry) => entry.id === repositoryId);
    if (!session) {
      throw new Error(`Unknown planner session: ${sessionId}`);
    }
    if (!repository) {
      throw new Error(`Unknown repository: ${repositoryId}`);
    }

    const product = ensureSelectedProductDraft(session, args);
    const plannerModule = createDraftNode(session, "module", "Interactive Planner", product.id, {
      description: "Conversational planning, design staging, approval flow, and trace inspection.",
    });
    const repoModule = createDraftNode(session, "module", "Repository Intelligence", product.id, {
      description: "Repository registration, reverse engineering, and code-aware planning expansion.",
    });
    const plannerCapability = createDraftNode(session, "capability", "Draft Tree Editing", plannerModule.id, {
      description: "Select, rename, expand, and refine staged design nodes before approval.",
    });
    createDraftNode(session, "work_item", "Add repo analysis entrypoint", repoModule.id, {
      description: "Let users register a repository and stage a reverse-engineered planning tree.",
    });
    createDraftNode(session, "work_item", "Improve design tree ergonomics", plannerCapability.id, {
      description: "Tighten the planner workspace so design editing feels like a real planning surface.",
    });
    session.selected_draft_node_id = product.id;
    return createPlannerResponse(
      session,
      `I analyzed the ${repository.name} repository and staged repository-informed modules under ${product.label} so you can refine them before approval.`,
      [
        {
          type: "create_module",
          target: { productName: product.label },
          name: plannerModule.label,
          description: plannerModule.data.description,
        },
        {
          type: "create_module",
          target: { productName: product.label },
          name: repoModule.label,
          description: repoModule.data.description,
        },
      ],
      ["Updated the design plan from repository analysis."],
      product.id,
    );
  }

  function renamePlannerDraftNode(args) {
    const sessionId = getArg(args, "sessionId", "session_id");
    const nodeId = getArg(args, "nodeId", "node_id");
    const name = String(getArg(args, "name") ?? "").trim();
    const session = getPlannerSession(sessionId);
    if (!session) {
      throw new Error(`Unknown planner session: ${sessionId}`);
    }
    const node = findDraftNodeById(session, nodeId);
    if (!node) {
      throw new Error("Draft node not found");
    }
    if (!name) {
      throw new Error("Draft node name cannot be empty");
    }
    node.label = name;
    session.selected_draft_node_id = node.id;
    const action = node.type === "work_item"
      ? { type: "update_work_item", target: { workItemTitle: node.label }, fields: { title: name } }
      : { type: `update_${node.type}`, fields: { name } };
    return createPlannerResponse(
      session,
      `Renamed draft ${node.type.replace("_", " ")} to "${name}".`,
      [action],
      [`Renamed "${name}".`],
      node.id,
    );
  }

  function addPlannerDraftChild(args) {
    const sessionId = getArg(args, "sessionId", "session_id");
    const parentNodeId = getArg(args, "parentNodeId", "parent_node_id");
    const childType = String(getArg(args, "childType", "child_type") ?? "");
    const name = String(getArg(args, "name") ?? "").trim();
    const summary = String(getArg(args, "summary") ?? "").trim();
    const session = getPlannerSession(sessionId);
    if (!session) {
      throw new Error(`Unknown planner session: ${sessionId}`);
    }
    const parent = findDraftNodeById(session, parentNodeId);
    if (!parent) {
      throw new Error("Parent design node not found");
    }
    if (!name) {
      throw new Error("Draft child name cannot be empty");
    }
    const created = createDraftNode(session, childType, name, parent.id, {
      description: summary,
    });
    session.selected_draft_node_id = created.id;
    const action = childType === "work_item"
      ? { type: "create_work_item", title: name, description: summary || undefined }
      : { type: `create_${childType}`, name, description: summary || undefined };
    return createPlannerResponse(
      session,
      `Added draft ${childType.replace("_", " ")} "${name}" under "${parent.label}".`,
      [action],
      [`Added ${childType.replace("_", " ")} "${name}".`],
      created.id,
    );
  }

  function deletePlannerDraftNode(args) {
    const sessionId = getArg(args, "sessionId", "session_id");
    const nodeId = getArg(args, "nodeId", "node_id");
    const session = getPlannerSession(sessionId);
    if (!session) {
      throw new Error(`Unknown planner session: ${sessionId}`);
    }
    const node = findDraftNodeById(session, nodeId);
    if (!node) {
      throw new Error("Draft node not found");
    }

    const idsToDelete = [node.id];
    for (let index = 0; index < idsToDelete.length; index += 1) {
      const currentId = idsToDelete[index];
      Object.values(session.draftNodes).forEach((candidate) => {
        if (candidate.parentId === currentId) {
          idsToDelete.push(candidate.id);
        }
      });
    }
    if (node.parentId) {
      const parent = findDraftNodeById(session, node.parentId);
      if (parent) {
        parent.children = parent.children.filter((childId) => childId !== node.id);
      }
    } else {
      session.draftRootIds = session.draftRootIds.filter((rootId) => rootId !== node.id);
    }
    idsToDelete.forEach((id) => {
      delete session.draftNodes[id];
    });
    session.selected_draft_node_id = node.parentId ?? null;
    session.has_draft_plan = session.draftRootIds.length > 0;
    return createPlannerResponse(
      session,
      `Removed draft ${node.type.replace("_", " ")} "${node.label}".`,
      [{ type: node.type === "product" ? "archive_product" : `delete_${node.type}` }],
      [`Removed ${node.type.replace("_", " ")} "${node.label}".`],
      session.selected_draft_node_id,
    );
  }

  function getArg(args, ...keys) {
    for (const key of keys) {
      if (args && Object.prototype.hasOwnProperty.call(args, key)) {
        return args[key];
      }
    }
    return undefined;
  }

  function ok(value) {
    return Promise.resolve(deepClone(value));
  }

  function notImplemented(command) {
    return Promise.resolve(null);
  }

  window.__ARUVI_E2E__ = {
    invoke(command, args) {
      const state = getState();

      switch (command) {
        case "list_providers":
          return ok(state.providers);
        case "list_model_definitions":
          return ok(state.modelDefinitions);
        case "list_products":
          return ok(state.products);
        case "list_strategy_nodes":
          return ok(state.strategyNodes);
        case "create_strategy_node": {
          const parentNodeId = getArg(args, "parentNodeId", "parent_node_id") ?? null;
          const nodeKind = getArg(args, "nodeKind", "node_kind") ?? "strategic_area";
          const node = {
            id: nextId("strategy"),
            parent_node_id: parentNodeId,
            node_kind: nodeKind,
            name: getArg(args, "name") ?? "",
            description: getArg(args, "description") ?? "",
            owner_label: getArg(args, "ownerLabel", "owner_label") ?? "",
            sort_order: state.strategyNodes.filter((entry) => entry.parent_node_id === parentNodeId).length,
            created_at: FIXED_TIMESTAMP,
            updated_at: FIXED_TIMESTAMP,
          };
          state.strategyNodes.push(node);
          return ok(node);
        }
        case "update_strategy_node": {
          const id = getArg(args, "id");
          const node = state.strategyNodes.find((entry) => entry.id === id);
          if (!node) {
            throw new Error("Unknown strategy node");
          }
          const clearParent = Boolean(getArg(args, "clearParent", "clear_parent"));
          if (clearParent) {
            node.parent_node_id = null;
          } else if (getArg(args, "parentNodeId", "parent_node_id") !== undefined) {
            node.parent_node_id = getArg(args, "parentNodeId", "parent_node_id");
          }
          node.node_kind = getArg(args, "nodeKind", "node_kind") ?? node.node_kind;
          node.name = getArg(args, "name") ?? node.name;
          node.description = getArg(args, "description") ?? node.description;
          node.owner_label = getArg(args, "ownerLabel", "owner_label") ?? node.owner_label;
          node.updated_at = FIXED_TIMESTAMP;
          return ok(node);
        }
        case "delete_strategy_node": {
          const id = getArg(args, "id");
          const pending = [id];
          const idsToDelete = new Set();
          while (pending.length > 0) {
            const current = pending.pop();
            idsToDelete.add(current);
            state.strategyNodes
              .filter((node) => node.parent_node_id === current)
              .forEach((node) => pending.push(node.id));
          }
          state.strategyNodes = state.strategyNodes.filter((node) => !idsToDelete.has(node.id));
          state.productStrategyLinks = state.productStrategyLinks.filter((link) => !idsToDelete.has(link.strategy_node_id));
          return ok(null);
        }
        case "list_product_strategy_links":
          return ok(state.productStrategyLinks);
        case "link_product_to_strategy": {
          const productId = getArg(args, "productId", "product_id");
          const strategyNodeId = getArg(args, "strategyNodeId", "strategy_node_id");
          const isPrimary = Boolean(getArg(args, "isPrimary", "is_primary"));
          if (isPrimary) {
            state.productStrategyLinks.forEach((link) => {
              if (link.product_id === productId) {
                link.is_primary = false;
              }
            });
          }
          let link = state.productStrategyLinks.find((entry) => entry.product_id === productId && entry.strategy_node_id === strategyNodeId);
          if (!link) {
            link = {
              id: nextId("strategy-link"),
              product_id: productId,
              strategy_node_id: strategyNodeId,
              is_primary: isPrimary,
              created_at: FIXED_TIMESTAMP,
            };
            state.productStrategyLinks.push(link);
          } else {
            link.is_primary = isPrimary;
          }
          return ok(link);
        }
        case "unlink_product_from_strategy": {
          const productId = getArg(args, "productId", "product_id");
          const strategyNodeId = getArg(args, "strategyNodeId", "strategy_node_id");
          state.productStrategyLinks = state.productStrategyLinks.filter((link) => link.product_id !== productId || link.strategy_node_id !== strategyNodeId);
          return ok(null);
        }
        case "list_product_dependencies":
          return ok(state.productDependencies);
        case "list_product_references": {
          const scopeType = getArg(args, "scopeType", "scope_type");
          const scopeId = getArg(args, "scopeId", "scope_id");
          if (scopeType && scopeId) {
            return ok(state.productReferences.filter((reference) => reference.scope_type === scopeType && reference.scope_id === scopeId));
          }
          return ok(state.productReferences);
        }
        case "create_product_reference": {
          const reference = {
            id: nextId("reference"),
            scope_type: getArg(args, "scopeType", "scope_type"),
            scope_id: getArg(args, "scopeId", "scope_id"),
            title: getArg(args, "title"),
            reference_kind: getArg(args, "referenceKind", "reference_kind") ?? "note",
            uri: getArg(args, "uri") ?? "",
            content: getArg(args, "content") ?? "",
            created_at: FIXED_TIMESTAMP,
            updated_at: FIXED_TIMESTAMP,
          };
          state.productReferences.unshift(reference);
          return ok(reference);
        }
        case "delete_product_reference": {
          const id = getArg(args, "id");
          state.productReferences = state.productReferences.filter((reference) => reference.id !== id);
          return ok(null);
        }
        case "create_product_dependency": {
          const dependency = {
            id: nextId("dependency"),
            product_id: getArg(args, "productId", "product_id"),
            capability_id: getArg(args, "capabilityId", "capability_id") ?? null,
            depends_on_product_id: getArg(args, "dependsOnProductId", "depends_on_product_id"),
            depends_on_capability_id: getArg(args, "dependsOnCapabilityId", "depends_on_capability_id") ?? null,
            dependency_kind: getArg(args, "dependencyKind", "dependency_kind") ?? "platform",
            description: getArg(args, "description") ?? "",
            status: getArg(args, "status") ?? "active",
            created_at: FIXED_TIMESTAMP,
            updated_at: FIXED_TIMESTAMP,
          };
          state.productDependencies.push(dependency);
          return ok(dependency);
        }
        case "get_product_tree":
          return ok(buildProductTree(getArg(args, "productId", "product_id")));
        case "list_work_items":
          return ok(listWorkItemsFiltered(args || {}));
        case "create_work_item":
          return ok(createWorkItemFromArgs(args || {}));
        case "update_work_item":
          return ok(updateWorkItemFromArgs(args || {}));
        case "delete_work_item":
          return ok(deleteWorkItemFromArgs(args || {}));
        case "summarize_work_items_by_product":
          return ok(summarizeWorkItemsByProduct());
        case "create_planner_session_command":
          return ok(plannerSessionInfo(createPlannerSessionRecord(args || {})));
        case "update_planner_session_command": {
          const session = getPlannerSession(getArg(args, "sessionId", "session_id"));
          if (!session) {
            throw new Error("Unknown planner session");
          }
          session.provider_id = getArg(args, "providerId", "provider_id") ?? session.provider_id;
          session.model_name = getArg(args, "modelName", "model_name") ?? session.model_name;
          return ok(plannerSessionInfo(session));
        }
        case "clear_planner_pending_command": {
          const session = getPlannerSession(getArg(args, "sessionId", "session_id"));
          if (!session) {
            throw new Error("Unknown planner session");
          }
          session.has_pending_plan = false;
          session.pending_plan = null;
          session.draftNodes = {};
          session.draftRootIds = [];
          session.has_draft_plan = false;
          session.selected_draft_node_id = null;
          return ok(plannerSessionInfo(session));
        }
        case "submit_planner_turn_command":
          return ok(submitPlannerTurn(args || {}));
        case "submit_planner_voice_turn_command":
          return ok(submitPlannerVoiceTurn(args || {}));
        case "confirm_planner_plan_command":
          return ok(confirmPlannerPlan(args || {}));
        case "rename_planner_draft_node_command":
          return ok(renamePlannerDraftNode(args || {}));
        case "add_planner_draft_child_command":
          return ok(addPlannerDraftChild(args || {}));
        case "delete_planner_draft_node_command":
          return ok(deletePlannerDraftNode(args || {}));
        case "get_setting":
          return ok(state.settings[getArg(args, "key")] ?? null);
        case "set_setting":
          state.settings[getArg(args, "key")] = String(getArg(args, "value") ?? "");
          return ok(null);
        case "get_mobile_bridge_status": {
          const bindHost = state.settings["mobile.bind_host"] ?? "127.0.0.1";
          const bindPort = Number.parseInt(state.settings["mobile.bind_port"] ?? "8787", 10) || 8787;
          const detectedLanIp = "192.168.1.42";
          const lanReady = !["127.0.0.1", "localhost", "::1"].includes(bindHost);
          return ok({
            bind_host: bindHost,
            bind_port: bindPort,
            host_source: state.settings["mobile.bind_host"] ? "settings" : "default",
            port_source: state.settings["mobile.bind_port"] ? "settings" : "default",
            bind_scope: lanReady ? (["0.0.0.0", "::"].includes(bindHost) ? "lan" : "custom") : "localhost-only",
            detected_lan_ip: detectedLanIp,
            desktop_base_url: `http://${bindHost === "0.0.0.0" ? "127.0.0.1" : bindHost}:${bindPort}`,
            phone_base_url: lanReady ? `http://${bindHost === "0.0.0.0" ? detectedLanIp : bindHost}:${bindPort}` : null,
            lan_ready: lanReady,
            bind_changes_require_restart: true,
            env_overrides_settings: false,
            guidance: lanReady
              ? "Use the phone base URL from the same Wi-Fi network."
              : `Set mobile.bind_host to 0.0.0.0 and restart, then connect the iPhone to http://${detectedLanIp}:${bindPort}.`,
          });
        }
        case "get_database_health":
          return ok({
            applied_migrations: 13,
            latest_version: 13,
            migrations: [{ version: 13, description: "planner draft state", success: true, installed_on: FIXED_TIMESTAMP }],
          });
        case "get_active_database_path":
          return ok("/tmp/aruvi-studio-e2e.db");
        case "get_database_path_override":
          return ok(null);
        case "set_database_path_override":
        case "clear_database_path_override":
        case "seed_example_products":
          return ok(null);
        case "list_repositories":
          return ok(state.repositories);
        case "list_repository_tree":
          return ok([]);
        case "resolve_repository_for_scope":
        case "resolve_repository_for_work_item":
          return ok(null);
        case "list_agent_definitions":
        case "list_agent_model_bindings":
        case "list_agent_teams":
        case "list_team_memberships":
        case "list_team_assignments":
        case "list_skills":
        case "list_agent_skill_links":
        case "list_team_skill_links":
        case "list_workflow_stage_policies":
        case "list_agent_runs_for_workflow":
        case "get_workflow_history":
        case "get_work_item_approvals":
        case "list_work_item_artifacts":
        case "list_work_item_findings":
          return ok([]);
        case "get_sub_work_items": {
          const workItemId = String(getArg(args, "workItemId", "work_item_id") ?? "");
          return ok(state.workItems.filter((entry) => entry.parent_work_item_id === workItemId));
        }
        case "list_external_cli_runs_for_work_item": {
          const workItemId = String(getArg(args, "workItemId", "work_item_id") ?? "");
          return ok(state.externalCliRuns.filter((run) => !workItemId || run.work_item_id === workItemId));
        }
        case "list_external_cli_run_events": {
          const runId = String(getArg(args, "runId", "run_id") ?? "");
          const limit = Number(getArg(args, "limit") ?? 500);
          return ok((state.externalCliRunEvents ?? [])
            .filter((event) => !runId || event.run_id === runId)
            .sort((a, b) => a.sequence - b.sequence)
            .slice(-Math.max(1, Math.min(limit, 2000))));
        }
        case "get_latest_workflow_run_for_work_item":
        case "get_workflow_run":
        case "get_work_item":
          return ok(null);
        case "read_artifact_content":
          return ok("");
        case "route_planner_contact_command":
          return ok({ channel: "whatsapp", status: "sent", reason: "Mock router chose WhatsApp for this request." });
        case "send_twilio_whatsapp_message":
        case "start_twilio_voice_call":
        case "speak_text_natively_command":
          return ok(null);
        case "transcribe_audio_command":
          return ok({ transcript: "Add voice-driven planning for the selected node" });
        case "browse_for_local_model_file":
          return ok("/tmp/mock-models/ggml-base.en.bin");
        case "register_local_runtime_model_command":
        case "install_managed_local_model_command":
          return ok({
            file_path: getArg(args, "modelPath") ?? "/tmp/mock-models/ggml-base.en.bin",
            downloaded: command === "install_managed_local_model_command",
            provider: {
              id: `provider-${slugify(getArg(args, "providerName") ?? "local-runtime")}`,
              name: getArg(args, "providerName") ?? "Whisper.cpp Base.en (Local)",
              provider_type: "local_runtime",
              base_url: getArg(args, "modelPath") ?? "/tmp/mock-models/ggml-base.en.bin",
              auth_secret_ref: null,
              enabled: true,
              created_at: FIXED_TIMESTAMP,
              updated_at: FIXED_TIMESTAMP,
            },
            model_definition: {
              id: `model-${slugify(getArg(args, "modelName") ?? "whisper-base-en")}`,
              provider_id: `provider-${slugify(getArg(args, "providerName") ?? "local-runtime")}`,
              name: getArg(args, "modelName") ?? "whisper-base.en",
              context_window: null,
              capability_tags: ["speech_to_text", "transcription", "audio", "local_runtime"],
              notes: getArg(args, "notes") ?? "",
              enabled: true,
              created_at: FIXED_TIMESTAMP,
              updated_at: FIXED_TIMESTAMP,
            },
          });
        case "run_model_chat_completion":
          return ok({ content: "{\"type\":\"final\",\"assistant_response\":\"mock\",\"needs_confirmation\":false,\"clarification_question\":null,\"actions\":[]}" });
        case "start_model_chat_stream":
          return ok("mock-chat-stream");
        case "register_repository":
          return ok(registerRepository(args || {}));
        case "analyze_repository_for_planner_command":
          return ok(analyzeRepositoryForPlanner(args || {}));
        case "browse_for_repository_path":
          return ok("/tmp/mock-repository");
        case "export_product_overview_html":
          return ok(`/tmp/aruvi-e2e/${getArg(args, "fileName", "file_name") ?? "export.html"}`);
        case "reveal_in_finder":
          return ok(null);
        case "delete_repository":
        case "attach_repository":
        case "create_local_workspace":
        case "read_repository_file":
        case "write_repository_file":
        case "apply_repository_patch":
        case "get_repository_file_sha256":
        case "create_product":
        case "get_product":
        case "update_product":
        case "archive_product":
        case "create_module":
        case "list_modules":
        case "update_module":
        case "delete_module":
        case "reorder_modules":
        case "create_capability":
        case "list_capabilities":
        case "update_capability":
        case "delete_capability":
        case "reorder_capabilities":
        case "reorder_work_items":
        case "approve_work_item":
        case "reject_work_item":
        case "approve_work_item_plan":
        case "reject_work_item_plan":
        case "approve_work_item_test_review":
        case "start_work_item_workflow":
          return notImplemented(command);
        case "invoke_external_cli_for_work_item": {
          const provider = String(getArg(args, "provider") ?? "codex");
          const workItemId = String(getArg(args, "workItemId", "work_item_id") ?? "");
          const runId = nextId("external-cli-run");
          const run = {
            id: runId,
            work_item_id: workItemId,
            provider,
            label: provider === "codex" ? "Codex CLI" : provider === "claude" ? "Claude Code CLI" : provider === "cursor" ? "Cursor Agent CLI" : "GitHub Copilot CLI",
            command: provider === "copilot" ? "copilot" : provider === "cursor" ? "cursor-agent" : provider,
            args: provider === "codex"
              ? ["exec", "--ignore-user-config", "--sandbox", "workspace-write", "--cd", "/tmp/mock-repository", "Mock work item prompt"]
              : provider === "copilot"
                ? ["--autopilot", "--no-ask-user", "--max-autopilot-continues", "10", "-s", "--allow-tool", "shell(git:*),shell(npm:*),shell(cargo:*),shell(rg:*),shell(ls:*),shell(cat:*),shell(sed:*),write", "-p", "Mock work item prompt"]
                : ["-p", "Mock work item prompt"],
            prompt: "Mock work item prompt",
            cwd: "/tmp/mock-repository",
            status: "completed",
            exit_code: 0,
            duration_ms: 25,
            stdout_chars: 32,
            stderr_chars: 0,
            session_log_path: `/tmp/aruvi-external-cli/${runId}/session.log`,
            output_artifact_id: null,
            error_message: null,
            started_at: FIXED_TIMESTAMP,
            ended_at: FIXED_TIMESTAMP,
            created_at: FIXED_TIMESTAMP,
          };
          state.externalCliRuns.unshift(run);
          state.externalCliRunEvents.push(
            {
              id: nextId("external-cli-event"),
              run_id: runId,
              work_item_id: workItemId,
              stream: "lifecycle",
              message: `Starting ${run.label}.`,
              sequence: 1,
              created_at: FIXED_TIMESTAMP,
            },
            {
              id: nextId("external-cli-event"),
              run_id: runId,
              work_item_id: workItemId,
              stream: "stdout",
              message: "Mock external CLI completed.",
              sequence: 2,
              created_at: FIXED_TIMESTAMP,
            },
            {
              id: nextId("external-cli-event"),
              run_id: runId,
              work_item_id: workItemId,
              stream: "lifecycle",
              message: "Captured output artifact.",
              sequence: 3,
              created_at: FIXED_TIMESTAMP,
            },
          );
          return ok(run);
        }
        case "handle_workflow_user_action":
        case "mark_workflow_run_failed":
        case "restart_workflow_run":
        case "create_provider":
        case "update_provider":
        case "delete_provider":
        case "create_model_definition":
        case "update_model_definition":
        case "delete_model_definition":
        case "create_agent_definition":
        case "update_agent_definition":
        case "delete_agent_definition":
        case "create_agent_team":
        case "update_agent_team":
        case "delete_agent_team":
        case "add_team_member":
        case "remove_team_member":
        case "assign_team_scope":
        case "remove_team_assignment":
        case "create_skill":
        case "update_skill":
        case "delete_skill":
        case "link_skill_to_agent":
        case "unlink_skill_from_agent":
        case "link_skill_to_team":
        case "unlink_skill_from_team":
        case "set_primary_agent_model_binding":
        case "upsert_workflow_stage_policy":
        case "delete_workflow_stage_policy":
          return notImplemented(command);
        default:
          return ok(null);
      }
    },
  };
})();
