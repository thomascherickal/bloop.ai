import React, { useRef, useState } from 'react';
import { ChevronDownFilled, ChevronUpFilled } from '../../../icons';
import ContextMenu, { ContextMenuItem } from '../../ContextMenu';
import Button from '../../Button';
import { useOnClickOutside } from '../../../hooks/useOnClickOutsideHook';

type Props = {
  items: ContextMenuItem[];
  hint?: string;
  icon: React.ReactElement;
  dropdownBtnClassName?: string;
  noChevron?: boolean;
  btnVariant?: 'primary' | 'secondary' | 'tertiary';
};

const Dropdown = ({
  items,
  hint,
  icon,
  dropdownBtnClassName,
  noChevron,
  btnVariant = 'tertiary',
}: Props) => {
  const [visible, setVisibility] = useState(false);
  const ref = useRef(null);
  useOnClickOutside(ref, () => setVisibility(false));

  return (
    <div className="relative" ref={ref}>
      <ContextMenu
        items={items}
        visible={visible}
        title={hint}
        handleClose={() => setVisibility(false)}
        closeOnClickOutside={false}
      >
        <Button
          variant={btnVariant}
          id="dropdownDefault"
          data-dropdown-toggle="dropdown"
          className={`${visible ? 'text-gray-50' : ''} ${
            dropdownBtnClassName || ''
          }`}
          onClick={() => setVisibility(!visible)}
        >
          {icon}
          {noChevron ? null : visible ? (
            <ChevronUpFilled />
          ) : (
            <ChevronDownFilled />
          )}
        </Button>
      </ContextMenu>
    </div>
  );
};

export default Dropdown;