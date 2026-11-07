const test = require('node:test');
const assert = require('node:assert/strict');
const protocol = require('./browser_protocol.cjs');

test('command delivery waits for the exact eligible instances to finish loading', () => {
    const frames = [{ elementInstanceId: 'one', ready: true }, { elementInstanceId: 'two', ready: false }];
    assert.deepEqual(protocol.missingDeliveryInstances(frames, ['one', 'two', 'missing']), ['two', 'missing']);
    assert.deepEqual(protocol.missingDeliveryInstances(frames, ['one']), []);
    assert.deepEqual(protocol.missingDeliveryInstances(frames, undefined), []);
});

test('targets one placed instance or every instance in a package', () => {
    const frames = [
        { elementInstanceId: 'placed-a', packageId: 'alerts' },
        { elementInstanceId: 'placed-b', packageId: 'alerts' },
        { elementInstanceId: 'placed-c', packageId: 'goals' }
    ];
    assert.deepEqual(
        [...protocol.targetedWidgetInstanceIds(frames, { type: 'widget', widgetId: 'placed-a' })],
        ['placed-a']
    );
    assert.deepEqual(
        [...protocol.targetedWidgetInstanceIds(frames, { type: 'widget', widgetId: 'alerts' })],
        ['placed-a', 'placed-b']
    );
});

test('host-accepted dedup survives source recreation while rendered work remains replayable', () => {
    const applied = new Map();
    const hostCommand = { commandId: 'host-1', deliveryRevision: 2, acknowledgementMode: 'hostAccepted' };
    protocol.rememberApplied(applied, hostCommand.commandId, hostCommand.deliveryRevision, 500);

    const recreatedSourceLedger = new Map(JSON.parse(JSON.stringify([...applied])));
    assert.equal(protocol.shouldSuppressHostAccepted(recreatedSourceLedger, hostCommand), true);
    assert.equal(protocol.shouldSuppressHostAccepted(recreatedSourceLedger, {
        ...hostCommand,
        deliveryRevision: 3
    }), false);
    assert.equal(protocol.shouldSuppressHostAccepted(recreatedSourceLedger, {
        ...hostCommand,
        acknowledgementMode: 'widgetRendered'
    }), false);
});

test('rendered receipts reject unrelated and reordered instances and complete exactly once', () => {
    const pending = { revision: 4, remaining: new Set(['placed-a', 'placed-b']) };
    assert.deepEqual(protocol.acceptRenderedReceipt(pending, 'placed-a', 3), {
        accepted: false,
        completed: false
    });
    assert.deepEqual(protocol.acceptRenderedReceipt(pending, 'placed-c', 4), {
        accepted: false,
        completed: false
    });
    assert.deepEqual(protocol.acceptRenderedReceipt(pending, 'placed-b', 4), {
        accepted: true,
        completed: false
    });
    assert.deepEqual(protocol.acceptRenderedReceipt(pending, 'placed-a', 4), {
        accepted: true,
        completed: true
    });
    assert.deepEqual(protocol.acceptRenderedReceipt(pending, 'placed-a', 4), {
        accepted: false,
        completed: false
    });
});

test('deduplication ledger stays bounded', () => {
    const applied = new Map();
    for (let revision = 1; revision <= 4; revision += 1) {
        protocol.rememberApplied(applied, `command-${revision}`, revision, 3);
    }
    assert.deepEqual([...applied.keys()], ['command-2', 'command-3', 'command-4']);
});
