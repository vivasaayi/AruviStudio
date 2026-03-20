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
      created_at: FIXED_TIMESTAMP,
      updated_at: FIXED_TIMESTAMP,
    };
  }

  function createState() {
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
          "example-product-calculator",
          "Calculator",
          "A staged React calculator used to pressure-test implementation and validation agents.",
          "Ship calculator outcomes one by one and verify the full autonomous delivery loop.",
          [
            "Validate coding agents against a familiar React app",
            "Exercise testing agents on incremental mathematical outcomes",
          ],
          ["example_product", "seeded_catalog", "react", "calculator"],
        ),
      ],
      modules: [],
      capabilities: [],
      workItems: [],
      repositories: [],
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
    return { product, modules };
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
      if (filters?.status && item.status !== filters.status) {
        return false;
      }
      return true;
    });
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

  function ensureHotelDraft(session) {
    const existing = findDraftNodeByLabel(session, "Hotel Management System", "product");
    if (existing) {
      return existing;
    }

    const product = createDraftNode(session, "product", "Hotel Management System", null, {
      description: "A comprehensive system for managing hotel operations including reservations, guest services, housekeeping, billing, and reporting.",
      vision: "Streamline hotel operations for small to mid-sized hotels with a unified, staff-friendly operating platform.",
      goals: [
        "Centralize reservations and guest lifecycle management",
        "Coordinate housekeeping and room readiness",
        "Automate billing, folios, and operational reporting",
      ],
      tags: ["hospitality", "management", "operations"],
    });

    const reservations = createDraftNode(session, "module", "Reservations & Booking", product.id, {
      description: "Handles room reservations, availability calendars, booking modifications, and cancellations.",
    });
    const guestManagement = createDraftNode(session, "module", "Guest Management", product.id, {
      description: "Manages guest profiles, check-in/out processes, service requests, and communication.",
    });
    const roomInventory = createDraftNode(session, "module", "Room & Inventory", product.id, {
      description: "Tracks room status, housekeeping, maintenance, and inventory of amenities.",
    });
    const billing = createDraftNode(session, "module", "Billing & Payments", product.id, {
      description: "Handles invoicing, payment processing, folio management, and financial reporting.",
    });

    const reservationBooking = createDraftNode(session, "capability", "Reservation Booking", reservations.id, {
      description: "Allow staff to create and manage reservations with date, room, and rate selection.",
    });
    createDraftNode(session, "work_item", "Build Reservation Creation UI", reservationBooking.id, {
      description: "Create a staff-facing reservation flow with date selection, room availability, and rate plan support.",
    });

    const guestProfiles = createDraftNode(session, "capability", "Guest Profile Management", guestManagement.id, {
      description: "Create, view, and update guest profiles with stay preferences and notes.",
    });
    createDraftNode(session, "work_item", "Implement Guest Profile CRUD", guestProfiles.id, {
      description: "Build backend and frontend CRUD for guest profiles and preference capture.",
    });

    const roomStatus = createDraftNode(session, "capability", "Room Availability Dashboard", roomInventory.id, {
      description: "Visualize room status and readiness across the property.",
    });
    createDraftNode(session, "work_item", "Develop Room Status Dashboard", roomStatus.id, {
      description: "Implement a dashboard with occupancy, cleaning, and maintenance state.",
    });

    const invoiceGeneration = createDraftNode(session, "capability", "Invoice Generation", billing.id, {
      description: "Create itemized invoices for stays, taxes, and incidentals.",
    });
    createDraftNode(session, "work_item", "Design Invoice Template", invoiceGeneration.id, {
      description: "Design a printable and email-ready invoice template with line items and taxes.",
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

    const resolvedProduct = productNode ?? ensureHotelDraft(session);
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
      message: "I expanded the draft with a Notifications & Messaging module, including guest preference management so you can handle email and WhatsApp communication cleanly.",
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

    const selectedNode = findDraftNodeById(session, session.selected_draft_node_id);
    const normalizedInput = userInput.toLowerCase();

    if (normalizedInput.includes("hotel management")) {
      const product = ensureHotelDraft(session);
      const actions = [
        {
          type: "create_product",
          target: { productName: product.label },
          name: product.label,
          description: product.data.description,
          vision: product.data.vision,
          goals: product.data.goals,
          tags: product.data.tags,
        },
        ...product.children.map((moduleId) => {
          const moduleNode = session.draftNodes[moduleId];
          return {
            type: "create_module",
            target: { productName: product.label },
            name: moduleNode.label,
            description: moduleNode.data.description,
          };
        }),
      ];
      return createPlannerResponse(
        session,
        "I created a staged hotel management system draft with a product root, core operational modules, foundational capabilities, and initial work items. Use the draft tree to refine any branch before committing it.",
        actions,
        ["Updated the draft plan."],
        product.id,
        [
          { stage: "tool_call", title: "Requested tool list_products", detail: "{\n  \"tool\": \"list_products\"\n}" },
          { stage: "tool_result", title: "Tool result list_products", detail: JSON.stringify(getState().products, null, 2) },
        ],
      );
    }

    if ((normalizedInput.includes("enhance") || normalizedInput.includes("expand")) && selectedNode?.type === "module") {
      const result = enhanceSelectedModule(session, selectedNode);
      return createPlannerResponse(session, result.message, result.actions, ["Updated the draft plan."], result.selectedNodeId);
    }

    if ((normalizedInput.includes("add work items") || normalizedInput.includes("implement") || normalizedInput.includes("break this down")) && selectedNode?.type === "capability") {
      const result = enhanceSelectedCapability(session, selectedNode);
      return createPlannerResponse(session, result.message, result.actions, ["Updated the draft plan."], result.selectedNodeId);
    }

    if ((normalizedInput.includes("revise") || normalizedInput.includes("consent")) && selectedNode?.type === "work_item") {
      const result = reviseSelectedWorkItem(session, selectedNode);
      return createPlannerResponse(session, result.message, result.actions, ["Updated the draft plan."], result.selectedNodeId);
    }

    if (normalizedInput.includes("email") || normalizedInput.includes("whatsapp") || normalizedInput.includes("notification")) {
      const result = addNotificationsToProduct(session, selectedNode);
      return createPlannerResponse(session, result.message, result.actions, ["Updated the draft plan."], result.selectedNodeId);
    }

    return {
      session_id: session.session_id,
      status: "clarification",
      assistant_message: "I need a bit more detail. Select a draft node or tell me whether you want to expand the product, a module, a capability, or a work item.",
      pending_plan: {
        assistant_response: "I need a bit more detail.",
        needs_confirmation: false,
        clarification_question: "Tell me which part of the draft you want to refine next.",
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
      assistant_message: "Committed draft plan.",
      pending_plan: null,
      tree_nodes: null,
      draft_tree_nodes: [],
      selected_draft_node_id: null,
      execution_lines: ["Committed draft plan to the catalog."],
      execution_errors: [],
      trace_events: [
        {
          step: 1,
          stage: "execution",
          title: "Committed staged draft",
          detail: "The staged product tree was persisted into products, modules, capabilities, and work items.",
        },
      ],
    };
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
        case "get_product_tree":
          return ok(buildProductTree(getArg(args, "productId", "product_id")));
        case "list_work_items":
          return ok(listWorkItemsFiltered(args || {}));
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
        case "confirm_planner_plan_command":
          return ok(confirmPlannerPlan(args || {}));
        case "get_setting":
          return ok(state.settings[getArg(args, "key")] ?? null);
        case "set_setting":
          state.settings[getArg(args, "key")] = String(getArg(args, "value") ?? "");
          return ok(null);
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
        case "get_sub_work_items":
          return ok([]);
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
          return ok(null);
        case "run_model_chat_completion":
          return ok({ content: "{\"type\":\"final\",\"assistant_response\":\"mock\",\"needs_confirmation\":false,\"clarification_question\":null,\"actions\":[]}" });
        case "start_model_chat_stream":
          return ok("mock-chat-stream");
        case "register_repository":
        case "delete_repository":
        case "attach_repository":
        case "create_local_workspace":
        case "browse_for_repository_path":
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
        case "create_work_item":
        case "update_work_item":
        case "delete_work_item":
        case "reorder_work_items":
        case "approve_work_item":
        case "reject_work_item":
        case "approve_work_item_plan":
        case "reject_work_item_plan":
        case "approve_work_item_test_review":
        case "start_work_item_workflow":
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
