
type ComponentMode = 'rotary' | 'click/press'


export const useHandleDragComponent = (event: DragEvent, component_mode: ComponentMode, icon_element_id: string) => {
  
    // Create an image element or use an existing one
    const dragIcon = document.querySelector(`#${icon_element_id}`);
    
    if (!dragIcon) {
        console.warn('Drag icon element not found');
        return;
    }

    const rect = dragIcon.getBoundingClientRect();
    const xOffset = rect.width / 1; // Center the drag image
    const yOffset = rect.height / 1;
    
    event.dataTransfer?.setDragImage(dragIcon, xOffset, yOffset);

    const payload = {
        event: 'mode',
        data:{
            mode: component_mode
        }

    };

    event.dataTransfer?.setData("application/json", JSON.stringify(payload));
};
