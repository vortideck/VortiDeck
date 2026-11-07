(function installVortiDeckOverlayProtocol(root, factory) {
    const protocol = factory();
    root.VortiDeckOverlayProtocol = protocol;
    if (typeof module !== 'undefined' && module.exports) module.exports = protocol;
})(typeof globalThis !== 'undefined' ? globalThis : window, function createProtocol() {
    function frameMatchesTarget(frame, target) {
        if (!target || target.type !== 'widget') return true;
        return frame.elementInstanceId === target.widgetId || frame.packageId === target.widgetId;
    }

    function targetedWidgetInstanceIds(frames, target) {
        return new Set(frames
            .filter(frame => frameMatchesTarget(frame, target))
            .map(frame => frame.elementInstanceId)
            .filter(Boolean));
    }

    function missingDeliveryInstances(frames, eligibleIds) {
        if (!Array.isArray(eligibleIds)) return [];
        const present = new Set(frames.filter(frame => frame.ready).map(frame => frame.elementInstanceId));
        return eligibleIds.filter(id => !present.has(id));
    }

    function shouldSuppressHostAccepted(appliedCommands, command) {
        if (!command || !command.commandId || command.acknowledgementMode === 'widgetRendered') {
            return false;
        }
        return (appliedCommands.get(command.commandId) || 0) >= (command.deliveryRevision || 1);
    }

    function rememberApplied(appliedCommands, commandId, revision, maximumEntries) {
        appliedCommands.set(commandId, revision);
        const limit = maximumEntries || 500;
        while (appliedCommands.size > limit) {
            appliedCommands.delete(appliedCommands.keys().next().value);
        }
    }

    function acceptRenderedReceipt(pending, instanceId, appliedRevision) {
        if (!pending || appliedRevision !== pending.revision || !pending.remaining.has(instanceId)) {
            return { accepted: false, completed: false };
        }
        pending.remaining.delete(instanceId);
        return { accepted: true, completed: pending.remaining.size === 0 };
    }

    return {
        frameMatchesTarget,
        missingDeliveryInstances,
        targetedWidgetInstanceIds,
        shouldSuppressHostAccepted,
        rememberApplied,
        acceptRenderedReceipt
    };
});
