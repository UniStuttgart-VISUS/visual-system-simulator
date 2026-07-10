// Inject mockup Activity when running outside of Android app, e.g., for development.
var Activity;
if (typeof Activity === "undefined") {
    Activity = {
        querySettings: function () {
            let mockData = [{
                "MockNodeA": {
                    "MockParamA1": "A String",
                    "MockParamA2": 42,
                    "MockParamA3": true,
                    "MockParamA4": "Another String",
                    "MockParamA5": {value: 0.5, min: 0.0, max: 1.0}
                }
            }, {
                "MockNodeB": {
                    "MockParamB1": true,
                    "MockParamB2": false,
                    "MockParamB3": true,
                    "MockParamB4": ""
                }
            }];
            console.info("Querying Settings", mockData);
            return JSON.stringify(mockData);
        },
        postSettings: function (jsonString) {
            console.assert(typeof jsonString === "string");
            console.info("Posting Settings", JSON.parse(jsonString));
            return;
        }
    };
}

// Generate a label for an attribute.
function trAttribute(nodeName, attributeName) {
    return `${nodeName} / ${attributeName}`;
}

// Build a DOM subtree with event listeners for an attribute.
function buildAttributeInput(nodeName, attributeName, attributeValue, updateValue) {
    let attributeId = `sp__${nodeName}__${attributeName}`;
    var elAttribute;
    if (typeof attributeValue == "boolean") {
        elAttribute = $(`
            <label for="${attributeId}">
                <small>${trAttribute(nodeName, attributeName)}</small>
                <input type="checkbox" id="${attributeId}" name="${attributeId}" role="switch" ${attributeValue ? "checked" : ""}>
            </label>`);
    } else if (typeof attributeValue == "number") {
        elAttribute = $(`
            <label for="${attributeId}">
                <small>${trAttribute(nodeName, attributeName)}</small>
                <input type="number" id="${attributeId}" name="${attributeId}" value="${attributeValue}">
            </label>`);
    } else if (typeof (typeof attributeValue !== "undefined" && attributeValue !== null ? attributeValue.value : void 0) === "number") {
        let step = (attributeValue.max - attributeValue.min) / 100.0;
        elAttribute = $(`
            <label for="${attributeId}">
                <small>${trAttribute(nodeName, attributeName)}</small>
                <input type="range" id="${attributeId}" name="${attributeId}" value="${attributeValue.value}" 
                    min="${attributeValue.min}" max="${attributeValue.max}" step="${step}">
            </label>`);
    } else {
        elAttribute = $(`
            <label for="${attributeId}">
                <small>${trAttribute(nodeName, attributeName)}</small>
                <input type="text" id="${attributeId}" name="${attributeId}" value="${attributeValue}">
            </label>`);
    }
    elAttribute.find('input').on("input", function () {
        let type = $(this).attr('type');
        var value;
        if (type == "checkbox") {
            value = $(this).prop('checked');
        } else if (type == "number") {
            value = parseFloat($(this).val());
        } else {
            value = $(this).val();
        }
        updateValue(value)
    });
    return elAttribute;
}

// Build DOM subtree listeners for the settings panel.
function buildSettingsPanel(settings) {
    let elPanel = $(`<div class="settings-panel"><form autocomplete="off"></form></div>`);
    let elForm = elPanel.children('form');
    elPanel.settings = JSON.parse(JSON.stringify(settings));
    for (let flowIndex = 0; flowIndex < settings.length; ++flowIndex) {
        let flow = settings[flowIndex];
        for (let nodeName in flow) {
            let elAttributeSet = $(`<fieldset></fieldset>`);
            let node = flow[nodeName];
            for (let attributeName in node) {
                let attributeValue = node[attributeName];
                let elAttribute = buildAttributeInput(
                    nodeName, attributeName, attributeValue,
                    function (flowIndex, nodeName, attributeName) {
                        return function (value) {
                            elPanel.settings[flowIndex][nodeName][attributeName] = value;
                            Activity.postSettings(JSON.stringify(elPanel.settings));
                        }
                    }(flowIndex, nodeName, attributeName));
                elAttributeSet.append(elAttribute);
            }
            elForm.append(elAttributeSet);
        }
    }
    return elPanel;
}

$(function () {
    const settings = JSON.parse(Activity.querySettings());
    $("body").append(buildSettingsPanel(settings));
});
